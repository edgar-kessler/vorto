//! Model downloads. Every file comes from the revision built into the app and must match the
//! size and SHA-256 built into the app before the model folder is published. Nothing the server
//! says about the files is trusted. Interrupted downloads continue where they stopped.
use crate::emit;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    os::windows::fs::OpenOptionsExt,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use vorto::{data, protocol::Event};

/// Attempts per file before a dropped connection becomes an error the user sees.
const ATTEMPTS: u32 = 5;

pub fn download(root: &Path, id: &str) -> Result<()> {
    let entry = data::model(id).context("Unknown model")?;
    let total: u64 = entry.files.iter().map(|f| f.size).sum();
    let models = root.join("models");
    fs::create_dir_all(&models)?;
    // One download of a model at a time, also across engines. Windows releases the lock
    // when this process ends, however it ends.
    let _lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(models.join(format!(".{id}.lock")))
        .map_err(|_| anyhow::anyhow!("{} is already downloading.", entry.name))?;
    let partial = models.join(format!(".{id}.partial"));
    // Files from another revision of the model can't be continued.
    let revision = partial.join("revision");
    if partial.exists() && fs::read_to_string(&revision).ok().as_deref() != Some(entry.revision) {
        fs::remove_dir_all(&partial)?;
    }
    fs::create_dir_all(&partial)?;
    fs::write(&revision, entry.revision)?;
    let verified = partial.join("verified");
    if fs::read_to_string(&verified).ok().as_deref() != Some(entry.revision) {
        let present: u64 = entry
            .files
            .iter()
            .map(|f| {
                fs::metadata(partial.join(f.name))
                    .map_or(0, |m| m.len())
                    .min(f.size)
            })
            .sum();
        ensure_space(&models, total - present, entry.name)?;
        let agent = agent(entry.name);
        let mut progress = Progress {
            name: entry.name,
            total,
            last: Instant::now(),
        };
        let mut done = 0u64;
        let mut buffer = vec![0u8; 256 * 1024];
        for file in entry.files {
            let path = partial.join(file.name);
            let url = format!(
                "https://huggingface.co/{}/resolve/{}/{}",
                entry.repo, entry.revision, file.name
            );
            let mut state = prefix(&path, file.size, &mut buffer, |n| progress.report(done + n))?;
            let mut attempt = 1;
            while state.count < file.size {
                match fetch(
                    &agent,
                    &url,
                    &path,
                    file.size,
                    &mut state,
                    &mut buffer,
                    |n| progress.report(done + n),
                ) {
                    Ok(()) => {}
                    Err(Failure::Retry(error)) if attempt < ATTEMPTS => {
                        attempt += 1;
                        progress.say(
                            &format!("Connection lost, trying again ({attempt} of {ATTEMPTS})"),
                            done + state.count,
                        );
                        std::thread::sleep(Duration::from_secs(2 * attempt as u64));
                        drop(error);
                    }
                    Err(Failure::Retry(error) | Failure::Final(error)) => return Err(error),
                }
            }
            if format!("{:x}", state.hasher.finalize()) != file.sha256 {
                let _ = fs::remove_file(&path);
                bail!("The download didn't match the published model.");
            }
            done += file.size;
        }
        fs::write(&verified, entry.revision)?;
    }
    publish(&partial, &data::model_dir(root, id))
        .with_context(|| format!("Couldn't finish installing {}", entry.name))?;
    emit(Event::Downloaded);
    Ok(())
}

/// Moves the verified download into place. Antivirus or indexing tools may hold a file open for
/// a moment, so this tries a few times; a verified download is never thrown away here.
fn publish(partial: &Path, target: &Path) -> Result<()> {
    let mut last = None;
    for _ in 0..10 {
        let result = (|| -> std::io::Result<()> {
            if target.exists() {
                fs::remove_dir_all(target)?;
            }
            fs::rename(partial, target)
        })();
        match result {
            Ok(()) => return Ok(()),
            Err(error) => last = Some(error),
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err(last.expect("at least one attempt").into())
}

enum Failure {
    /// The connection dropped, timed out or the server was busy: worth another attempt.
    Retry(anyhow::Error),
    Final(anyhow::Error),
}
impl<E: Into<anyhow::Error>> From<E> for Failure {
    fn from(error: E) -> Self {
        Failure::Final(error.into())
    }
}

struct Progress {
    name: &'static str,
    total: u64,
    last: Instant,
}
impl Progress {
    fn report(&mut self, done: u64) {
        if self.last.elapsed() > Duration::from_millis(150) {
            let text = format!(
                "{:.0} of {:.0} MB",
                done as f64 / 1e6,
                self.total as f64 / 1e6
            );
            self.say(&text, done);
        }
    }
    /// Every event also tells the app the download is still alive.
    fn say(&mut self, text: &str, done: u64) {
        emit(Event::Progress {
            detail: format!("Downloading {}  ·  {text}", self.name),
            fraction: done as f32 / self.total as f32,
        });
        self.last = Instant::now();
    }
}

/// How much of a file is on disk and verified so far.
struct State {
    count: u64,
    hasher: Sha256,
}

/// Hashes the part of the file already on disk, up to its expected size.
fn prefix(path: &Path, size: u64, buffer: &mut [u8], mut report: impl FnMut(u64)) -> Result<State> {
    let mut state = State {
        count: 0,
        hasher: Sha256::new(),
    };
    let Ok(mut existing) = fs::File::open(path) else {
        return Ok(state);
    };
    loop {
        let limit = buffer.len().min((size - state.count) as usize);
        let n = existing.read(&mut buffer[..limit])?;
        if n == 0 {
            break;
        }
        state.hasher.update(&buffer[..n]);
        state.count += n as u64;
        report(state.count);
    }
    Ok(state)
}

/// Continues one file from `state` until the server stops sending. The file is cut to exactly
/// the verified length first, so nothing unverified stays in it.
fn fetch(
    agent: &ureq::Agent,
    url: &str,
    path: &Path,
    size: u64,
    state: &mut State,
    buffer: &mut [u8],
    mut report: impl FnMut(u64),
) -> std::result::Result<(), Failure> {
    let mut request = agent.get(url);
    if state.count > 0 {
        request = request.set("Range", &format!("bytes={}-", state.count));
    }
    let response = match request.call() {
        Ok(response) => response,
        Err(ureq::Error::Status(401 | 404 | 410, _)) => {
            return Err(Failure::Final(anyhow::anyhow!(
                "This version of the model can't be downloaded any more. Update Vorto to get a current one."
            )))
        }
        Err(ureq::Error::Status(403, _)) => {
            return Err(Failure::Final(anyhow::anyhow!(
                "The download was blocked. A firewall or proxy on this network may not allow it."
            )))
        }
        Err(ureq::Error::Status(416, _)) => {
            // The file on disk is not a prefix the server knows. Start over.
            let _ = fs::remove_file(path);
            *state = State { count: 0, hasher: Sha256::new() };
            return Err(Failure::Retry(anyhow::anyhow!("The download had to start over.")));
        }
        Err(ureq::Error::Status(code, _)) if code == 408 || code == 429 || code >= 500 => {
            return Err(Failure::Retry(anyhow::anyhow!("The download server is busy ({code}). Try again later.")))
        }
        Err(ureq::Error::Status(code, _)) => {
            return Err(Failure::Final(anyhow::anyhow!("The download server answered with an error ({code}).")))
        }
        // A proxy that refuses the connection answers as a transport error, not a status. A sign-in
        // it asks for gives the same answer every time.
        Err(ureq::Error::Transport(t)) if t.kind() == ureq::ErrorKind::ProxyUnauthorized => {
            return Err(Failure::Final(anyhow::anyhow!(
                "The download was blocked. The proxy on this network needs a sign-in Vorto can't provide."
            )))
        }
        // Other refusals include a busy proxy (502 to 504), so they get the usual attempts.
        Err(ureq::Error::Transport(t)) if t.kind() == ureq::ErrorKind::ProxyConnect => {
            return Err(Failure::Retry(anyhow::anyhow!(
                "The download was blocked. A firewall or proxy on this network may not allow it."
            )))
        }
        Err(error) => return Err(Failure::Retry(network(error))),
    };
    if state.count > 0 && response.status() != 206 {
        // The server sent the whole file.
        *state = State {
            count: 0,
            hasher: Sha256::new(),
        };
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.set_len(state.count)?;
    file.seek(SeekFrom::Start(state.count))?;
    let mut output = BufWriter::new(file);
    let mut reader = response.into_reader();
    loop {
        let n = match reader.read(buffer) {
            Ok(n) => n,
            Err(error) => {
                // Keep what arrived for the next attempt.
                output.flush()?;
                return Err(Failure::Retry(network(error)));
            }
        };
        if n == 0 {
            break;
        }
        if state.count + n as u64 > size {
            drop(output);
            let _ = fs::remove_file(path);
            *state = State {
                count: 0,
                hasher: Sha256::new(),
            };
            return Err(Failure::Final(anyhow::anyhow!(
                "The download didn't match the published model."
            )));
        }
        output.write_all(&buffer[..n])?;
        state.hasher.update(&buffer[..n]);
        state.count += n as u64;
        report(state.count);
    }
    output
        .into_inner()
        .map_err(|e| e.into_error())?
        .sync_all()?;
    if state.count < size {
        return Err(Failure::Retry(anyhow::anyhow!(
            "The connection closed before the download finished."
        )));
    }
    Ok(())
}

fn network(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("The download was interrupted. Check your internet connection. ({error})")
}

fn agent(name: &'static str) -> ureq::Agent {
    emit(Event::Progress {
        detail: format!("Downloading {name}  ·  Connecting"),
        fraction: -1.0,
    });
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(30))
        .user_agent(concat!("Vorto/", env!("CARGO_PKG_VERSION")));
    // Windows' own TLS: it trusts the certificates Windows trusts, including a company's own.
    if let Ok(tls) = native_tls::TlsConnector::new() {
        builder = builder.tls_connector(Arc::new(tls));
    }
    let from_env = ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
        .iter()
        .any(|v| std::env::var_os(v).is_some());
    if from_env {
        builder = builder.try_proxy_from_env(true);
    } else if let Some(proxy) =
        crate::proxy::for_url("https://huggingface.co/").and_then(|p| ureq::Proxy::new(p).ok())
    {
        builder = builder.proxy(proxy);
    }
    builder.build()
}

fn ensure_space(dir: &Path, needed: u64, name: &str) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    let known = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0;
    // Room to spare for the file system and the rest of the PC.
    if known && free < needed + 200_000_000 {
        bail!(
            "There isn't enough free space for {name}. It needs about {:.1} GB, and {:.1} GB is free.",
            (needed + 200_000_000) as f64 / 1e9,
            free as f64 / 1e9
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::BufRead, net::TcpListener, thread};

    /// Serves `body` once per connection, honoring `Range: bytes=N-` unless `ranges` is false.
    fn serve(body: Vec<u8>, ranges: bool, connections: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming().take(connections) {
                let mut stream = stream.unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut from = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        from = value.trim().trim_end_matches('-').parse().unwrap();
                    }
                }
                let (status, part) = if ranges && from > 0 {
                    ("206 Partial Content", &body[from..])
                } else {
                    ("200 OK", &body[..])
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    part.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(part).unwrap();
            }
        });
        format!("http://{address}/model.bin")
    }

    /// `junk` is appended after the verified prefix was read, as a stray writer would.
    fn fetch_all(url: &str, path: &Path, size: u64, junk: &[u8]) -> State {
        let agent = ureq::AgentBuilder::new().build();
        let mut buffer = vec![0u8; 1024];
        let mut state = prefix(path, size, &mut buffer, |_| {}).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(junk)
            .unwrap();
        while state.count < size {
            if fetch(&agent, url, path, size, &mut state, &mut buffer, |_| {}).is_err() {
                panic!("fetch failed");
            }
        }
        state
    }

    #[test]
    fn resume_continues_after_the_verified_part() {
        let body: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        fs::write(&path, &body[..20_000]).unwrap();
        let url = serve(body.clone(), true, 1);
        let state = fetch_all(&url, &path, body.len() as u64, &[0xAA; 4_000]);
        assert_eq!(fs::read(&path).unwrap(), body);
        assert_eq!(
            format!("{:x}", state.hasher.finalize()),
            format!("{:x}", Sha256::digest(&body))
        );
    }

    #[test]
    fn a_server_without_ranges_sends_the_whole_file() {
        let body: Vec<u8> = (0..30_000u32).map(|i| (i % 13) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        fs::write(&path, &body[..10_000]).unwrap();
        let url = serve(body.clone(), false, 1);
        let state = fetch_all(&url, &path, body.len() as u64, &[]);
        assert_eq!(fs::read(&path).unwrap(), body);
        assert_eq!(
            format!("{:x}", state.hasher.finalize()),
            format!("{:x}", Sha256::digest(&body))
        );
    }

    #[test]
    fn a_proxy_that_refuses_is_not_retried() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let mut stream = listener.incoming().next().unwrap().unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n",
                )
                .unwrap();
        });
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .proxy(ureq::Proxy::new(format!("127.0.0.1:{port}")).unwrap())
            .build();
        let dir = tempfile::tempdir().unwrap();
        let mut state = State {
            count: 0,
            hasher: Sha256::new(),
        };
        let mut buffer = vec![0u8; 1024];
        let result = fetch(
            &agent,
            "https://huggingface.co/model.bin",
            &dir.path().join("model.bin"),
            1_000,
            &mut state,
            &mut buffer,
            |_| {},
        );
        assert!(matches!(result, Err(Failure::Final(_))));
    }
}
