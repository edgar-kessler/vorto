//! `vorto` on the command line: everything the settings window does, without a window. The
//! installer puts this program next to vorto.exe as vorto.com, which Windows prefers when you
//! type `vorto`. It edits the same settings.json, and a running Vorto picks up the change.
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command as Process, Stdio},
};
use vorto::{
    data::{self, Learned, Replacement, Settings, Store},
    protocol::{Command, Event, ENGINE_EXE},
};

const HELP: &str = "\
Vorto: say it, Vorto writes it.

  vorto start [--window]          Start Vorto: only the recording pill, or with its window
  vorto stop                      Quit Vorto
  vorto status                    What's running and how it's set up

  vorto settings                  All settings
  vorto settings get <key>        One setting, for example ai.enabled
  vorto settings set <key> <val>  Change one, for example: vorto settings set sounds false
  vorto shortcut [keys]           Show or set the shortcut: vorto shortcut \"Right Ctrl\"

  vorto models                    Voice models
  vorto models use <id>           Use a downloaded model
  vorto models download <id>      Download a model, and use it

  vorto words [add|remove <word>] Your dictionary
  vorto learned [forget <word>]   What Vorto learned from your corrections
  vorto replace [add <from> <to>|remove <from>]  Replacements
  vorto history [count]           Your last dictations

  vorto help | version
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    if let Err(error) = run(&args) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run(args: &[&str]) -> Result<()> {
    match args {
        [] | ["help" | "--help" | "-h"] => print!("{HELP}"),
        ["version" | "--version" | "-V"] => println!("{}", env!("CARGO_PKG_VERSION")),
        ["start"] => launch(&["--headless"])?,
        ["start", "--window"] => launch(&[])?,
        ["stop"] => launch(&["--quit"])?,
        ["status"] => status(&Store::open()?),
        ["settings"] => println!(
            "{}",
            serde_json::to_string_pretty(&view(&Store::open()?.settings)?)?
        ),
        ["settings", "get", key] => {
            println!("{}", show(lookup(&view(&Store::open()?.settings)?, key)?))
        }
        ["settings", "set", key, value] => {
            let mut store = Store::open()?;
            store.settings = with_setting(&store.settings, key, value)?;
            store.save()?;
            println!("{key} = {}", show(lookup(&view(&store.settings)?, key)?));
        }
        ["shortcut"] => println!("{}", shortcut_names(&Store::open()?.settings.hotkey)),
        ["shortcut", keys] => {
            let mut store = Store::open()?;
            store.settings.hotkey = parse_shortcut(keys)?;
            store.settings.validate();
            store.save()?;
            println!("{}", shortcut_names(&store.settings.hotkey));
        }
        ["models"] => list_models(&Store::open()?),
        ["models", "use", id] => {
            let mut store = Store::open()?;
            let model = known_model(id)?;
            if !data::installed(&store.root, id) {
                bail!(
                    "{} isn't downloaded yet. Try: vorto models download {id}",
                    model.name
                );
            }
            store.settings.model = model.id.into();
            store.save()?;
            println!("Using {}", model.name);
        }
        ["models", "download", id] => {
            known_model(id)?;
            let mut store = Store::open()?;
            download(&store.root, id)?;
            store.settings.model = (*id).into();
            store.save()?;
        }
        ["words"] => Store::open()?
            .settings
            .vocabulary
            .iter()
            .for_each(|w| println!("{w}")),
        ["words", "add", word] => {
            let mut store = Store::open()?;
            if !store
                .settings
                .vocabulary
                .iter()
                .any(|w| w.eq_ignore_ascii_case(word))
            {
                store.settings.vocabulary.push((*word).into());
            }
            store.save()?;
        }
        ["words", "remove", word] => {
            let mut store = Store::open()?;
            store
                .settings
                .vocabulary
                .retain(|w| !w.eq_ignore_ascii_case(word));
            store.save()?;
        }
        ["learned"] => {
            for Learned { from, to, count } in Store::open()?.settings.learned {
                println!("{from} -> {to} ({count}x)");
            }
        }
        ["learned", "forget", word] => {
            let mut store = Store::open()?;
            store
                .settings
                .learned
                .retain(|l| !l.from.eq_ignore_ascii_case(word) && !l.to.eq_ignore_ascii_case(word));
            store.save()?;
        }
        ["replace"] => {
            for Replacement { from, to } in Store::open()?.settings.replacements {
                println!("{from} -> {to}");
            }
        }
        ["replace", "add", from, to] => {
            let mut store = Store::open()?;
            store
                .settings
                .replacements
                .retain(|r| !r.from.eq_ignore_ascii_case(from));
            store.settings.replacements.push(Replacement {
                from: (*from).into(),
                to: (*to).into(),
            });
            store.save()?;
        }
        ["replace", "remove", from] => {
            let mut store = Store::open()?;
            store
                .settings
                .replacements
                .retain(|r| !r.from.eq_ignore_ascii_case(from));
            store.save()?;
        }
        ["history"] => history(&Store::open()?, 10),
        ["history", count] => history(
            &Store::open()?,
            count.parse().context("That's not a number.")?,
        ),
        _ => bail!("Unknown command. Try: vorto help"),
    }
    Ok(())
}

fn beside_me(name: &str) -> Result<PathBuf> {
    let mut me = std::env::current_exe()?;
    // install.sh links this program into ~/.local/bin on a Mac.
    if !cfg!(windows) {
        me = me.canonicalize()?;
    }
    Ok(me.with_file_name(name))
}

/// Hands over to the app itself. It runs once: a second start only passes on what it was asked.
fn launch(args: &[&str]) -> Result<()> {
    let app = beside_me(if cfg!(windows) { "vorto.exe" } else { "vorto-app" })?;
    Process::new(&app)
        .args(args)
        .spawn()
        .with_context(|| format!("Couldn't start {}", app.display()))?;
    Ok(())
}

fn running() -> bool {
    if cfg!(windows) {
        Process::new("tasklist")
            .args(["/FI", "IMAGENAME eq vorto.exe", "/NH"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("vorto.exe"))
    } else {
        Process::new("pgrep")
            .args(["-x", "vorto-app"])
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

fn status(store: &Store) {
    let settings = &store.settings;
    let model = data::model(&settings.model).map_or(settings.model.as_str(), |m| m.name);
    let ready = data::installed(&store.root, &settings.model);
    println!("Vorto       {}", env!("CARGO_PKG_VERSION"));
    println!("Running     {}", if running() { "yes" } else { "no" });
    println!(
        "Model       {model}{}",
        if ready { "" } else { " (not downloaded)" }
    );
    println!("Language    {}", settings.language);
    println!("Shortcut    {}", shortcut_names(&settings.hotkey));
    println!(
        "Dictionary  {} words, {} learned",
        settings.vocabulary.len(),
        settings.learned.len()
    );
    println!(
        "AI editing  {}",
        if settings.ai.enabled { "on" } else { "off" }
    );
    println!("Data        {}", store.root.display());
}

fn view(settings: &Settings) -> Result<Value> {
    Ok(serde_json::to_value(settings)?)
}

fn lookup<'a>(root: &'a Value, key: &str) -> Result<&'a Value> {
    key.split('.')
        .try_fold(root, |value, part| value.get(part))
        .with_context(|| format!("There's no setting called {key}. See: vorto settings"))
}

/// A string as itself, anything else as JSON.
fn show(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_string)
}

/// The settings with one changed. A value that doesn't fit, or that validation takes back, is
/// refused rather than quietly replaced.
fn with_setting(settings: &Settings, key: &str, value: &str) -> Result<Settings> {
    let mut root = view(settings)?;
    let mut slot = &mut root;
    for part in key.split('.') {
        slot = slot
            .get_mut(part)
            .with_context(|| format!("There's no setting called {key}. See: vorto settings"))?;
    }
    let parsed = serde_json::from_str::<Value>(value).unwrap_or_else(|_| json!(value));
    // Text settings stay text: `vorto settings set language 1` isn't a number there.
    let wanted = if slot.is_string() {
        json!(value)
    } else {
        parsed
    };
    let same_kind = std::mem::discriminant(slot) == std::mem::discriminant(&wanted);
    if !same_kind {
        bail!("{key} is {}, not {value}.", kind(slot));
    }
    *slot = wanted.clone();
    let mut changed: Settings = serde_json::from_value(root)?;
    changed.validate();
    if lookup(&view(&changed)?, key)? != &wanted {
        bail!("{value} isn't a value {key} can have.");
    }
    Ok(changed)
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "true or false",
        Value::Number(_) => "a number",
        Value::String(_) => "text",
        Value::Array(_) => "a list",
        _ => "a group of settings",
    }
}

// Virtual-key codes, as in ui/src/lib/keys.js.
const KEYS: &[(&str, u32)] = &[
    ("Ctrl", 0x11),
    ("Alt", 0x12),
    ("Shift", 0x10),
    ("Left Ctrl", 0xa2),
    ("Right Ctrl", 0xa3),
    ("Left Shift", 0xa0),
    ("Right Shift", 0xa1),
    ("Left Alt", 0xa4),
    ("Right Alt", 0xa5),
    ("Win", 0x5b),
    ("Menu", 0x5d),
    ("Space", 0x20),
    ("Enter", 0x0d),
    ("Tab", 0x09),
    ("Caps Lock", 0x14),
    ("Insert", 0x2d),
    ("Delete", 0x2e),
    ("Home", 0x24),
    ("End", 0x23),
    ("Page Up", 0x21),
    ("Page Down", 0x22),
    ("Pause", 0x13),
    ("Scroll Lock", 0x91),
];

fn key_code(name: &str) -> Option<u32> {
    let name = name.trim();
    if let Some((_, code)) = KEYS.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)) {
        return Some(*code);
    }
    if let Some(number) = name
        .strip_prefix(['F', 'f'])
        .and_then(|n| n.parse::<u32>().ok())
    {
        return (1..=24).contains(&number).then_some(0x6f + number);
    }
    let mut chars = name.chars();
    match (chars.next()?.to_ascii_uppercase(), chars.next()) {
        (c @ ('A'..='Z' | '0'..='9'), None) => Some(c as u32),
        _ => None,
    }
}

fn parse_shortcut(text: &str) -> Result<Vec<u32>> {
    let keys = text
        .split('+')
        .map(|name| {
            key_code(name).with_context(|| format!("I don't know the key {}.", name.trim()))
        })
        .collect::<Result<Vec<u32>>>()?;
    if keys.len() > 4 {
        bail!("A shortcut has at most four keys.");
    }
    Ok(keys)
}

fn shortcut_names(keys: &[u32]) -> String {
    keys.iter()
        .map(|&code| match KEYS.iter().find(|(_, c)| *c == code) {
            Some((name, _)) => name.to_string(),
            None if (0x70..=0x87).contains(&code) => format!("F{}", code - 0x6f),
            None => char::from_u32(code).map_or_else(|| code.to_string(), |c| c.to_string()),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn known_model(id: &str) -> Result<&'static data::Model> {
    data::model(id)
        .with_context(|| format!("There's no voice model called {id}. See: vorto models"))
}

fn list_models(store: &Store) {
    for model in data::MODELS {
        let mark = if store.settings.model == model.id {
            "*"
        } else if data::installed(&store.root, model.id) {
            "+"
        } else {
            " "
        };
        println!(
            "{mark} {:<16} {:<18} {:>4} MB  {}",
            model.id, model.name, model.download_mb, model.languages
        );
    }
    println!("\n* in use   + downloaded");
}

/// Lets the voice engine do it, as the app does.
fn download(root: &std::path::Path, id: &str) -> Result<()> {
    let engine = beside_me(ENGINE_EXE)?;
    let mut child = Process::new(&engine)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Couldn't start {}", engine.display()))?;
    let command = serde_json::to_string(&Command::Download {
        root: root.to_path_buf(),
        model: id.into(),
    })?;
    let mut input = child
        .stdin
        .take()
        .context("The voice engine has no input.")?;
    writeln!(input, "{command}")?;
    let output = child
        .stdout
        .take()
        .context("The voice engine has no output.")?;
    for line in BufReader::new(output).lines() {
        match serde_json::from_str::<Event>(&line?) {
            Ok(Event::Progress { detail, fraction }) if fraction >= 0.0 => {
                print!("\r{:>3} %  {detail}          ", (fraction * 100.0) as u32);
                std::io::stdout().flush()?;
            }
            Ok(Event::Downloaded) => {
                println!("\nDownloaded.");
                return Ok(());
            }
            Ok(Event::Error { detail }) => bail!(detail),
            _ => {}
        }
    }
    bail!("The voice engine stopped before the download finished.")
}

fn history(store: &Store, count: usize) {
    for entry in store.history.iter().take(count) {
        println!("{}  {}", entry.at, entry.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_setting_can_be_changed() {
        let changed = with_setting(&Settings::default(), "sounds", "false").unwrap();
        assert!(!changed.sounds);
        let changed = with_setting(&Settings::default(), "ai.enabled", "true").unwrap();
        assert!(changed.ai.enabled);
        let changed = with_setting(&Settings::default(), "theme", "dark").unwrap();
        assert_eq!(changed.theme, "dark");
    }
    #[test]
    fn a_wrong_setting_is_refused() {
        assert!(with_setting(&Settings::default(), "nope", "1").is_err());
        assert!(with_setting(&Settings::default(), "sounds", "maybe").is_err());
        assert!(with_setting(&Settings::default(), "theme", "purple").is_err());
    }
    #[test]
    fn shortcuts_read_and_write_key_names() {
        assert_eq!(parse_shortcut("Right Ctrl").unwrap(), [0xa3]);
        assert_eq!(parse_shortcut("left ctrl + space").unwrap(), [0xa2, 0x20]);
        assert_eq!(parse_shortcut("F9").unwrap(), [0x78]);
        assert!(parse_shortcut("Banana").is_err());
        assert_eq!(shortcut_names(&[0xa2, 0x20]), "Left Ctrl+Space");
        // Saved combinations don't tell left from right.
        assert_eq!(shortcut_names(&[0x11, 0x20]), "Ctrl+Space");
        assert_eq!(shortcut_names(&[0x78]), "F9");
        assert_eq!(shortcut_names(&[0x41]), "A");
    }
}
