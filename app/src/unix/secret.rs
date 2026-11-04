//! API keys for AI providers live in the Keychain on macOS and the Secret Service on Linux,
//! never in settings.json.
fn entry(provider: &str) -> Option<keyring::Entry> {
    keyring::Entry::new("Vorto", &format!("ai/{provider}")).ok()
}

pub fn read(provider: &str) -> Option<String> {
    entry(provider)?.get_password().ok()
}

pub fn write(provider: &str, key: &str) -> bool {
    if key.is_empty() {
        remove(provider);
        return true;
    }
    entry(provider).is_some_and(|e| e.set_password(key).is_ok())
}

pub fn remove(provider: &str) {
    if let Some(entry) = entry(provider) {
        let _ = entry.delete_credential();
    }
}
