//! Small diagnostic log in the data folder. Never records transcript text.
use std::{io::Write, path::PathBuf, sync::OnceLock};

static PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(root: &std::path::Path) {
    let path = root.join("vorto.log");
    // Keep the file small: start fresh when it grows past 1 MB.
    if path.metadata().is_ok_and(|m| m.len() > 1_000_000) {
        let _ = std::fs::remove_file(&path);
    }
    let _ = PATH.set(path);
}

pub fn write(message: impl AsRef<str>) {
    let Some(path) = PATH.get() else { return };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{} {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            message.as_ref()
        );
    }
}
