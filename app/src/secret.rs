//! API keys for AI providers live in Windows Credential Manager, encrypted for this Windows
//! account, never in settings.json.
use windows_sys::Win32::{
    Foundation::FILETIME,
    Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    },
};

fn target(provider: &str) -> Vec<u16> {
    format!("Vorto/ai/{provider}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub fn read(provider: &str) -> Option<String> {
    let name = target(provider);
    unsafe {
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        if CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut found) == 0 || found.is_null() {
            return None;
        }
        let credential = &*found;
        // An empty credential has no blob at all.
        let key = if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
            None
        } else {
            let bytes = std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            );
            String::from_utf8(bytes.to_vec()).ok()
        };
        CredFree(found as *const _);
        key.filter(|k| !k.is_empty())
    }
}

/// Stores the key, or removes it when `key` is empty.
pub fn write(provider: &str, key: &str) -> bool {
    let name = target(provider);
    let key = key.trim();
    unsafe {
        if key.is_empty() {
            CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0);
            return true;
        }
        let mut blob = key.as_bytes().to_vec();
        let mut user: Vec<u16> = "Vorto".encode_utf16().chain(Some(0)).collect();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_ptr() as *mut u16,
            Comment: std::ptr::null_mut(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: user.as_mut_ptr(),
        };
        CredWriteW(&credential, 0) != 0
    }
}

pub fn remove(provider: &str) {
    write(provider, "");
}
