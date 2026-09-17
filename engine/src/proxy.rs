//! The proxy Windows uses for a URL: the answer of a proxy script (PAC) or automatic detection,
//! as in company networks, or else the fixed proxy from the Internet settings.
use windows_sys::{
    core::PWSTR,
    Win32::{
        Foundation::{GetLastError, GlobalFree},
        Networking::WinHttp::{
            WinHttpCloseHandle, WinHttpGetIEProxyConfigForCurrentUser, WinHttpGetProxyForUrl,
            WinHttpOpen, WinHttpSetTimeouts, ERROR_WINHTTP_LOGIN_FAILURE,
            WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_AUTOPROXY_AUTO_DETECT,
            WINHTTP_AUTOPROXY_CONFIG_URL, WINHTTP_AUTOPROXY_OPTIONS, WINHTTP_AUTO_DETECT_TYPE_DHCP,
            WINHTTP_AUTO_DETECT_TYPE_DNS_A, WINHTTP_CURRENT_USER_IE_PROXY_CONFIG,
            WINHTTP_PROXY_INFO,
        },
    },
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Reads and frees a string WinHTTP allocated.
unsafe fn take(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let len = (0..).take_while(|&i| unsafe { *p.add(i) } != 0).count();
    let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) });
    unsafe { GlobalFree(p as _) };
    Some(text)
}

enum Automatic {
    /// The script or detection answered: this proxy, or None for a direct connection.
    Answer(Option<String>),
    /// No script found or it failed: the fixed proxy applies.
    Failed,
}

/// `host:port` for HTTPS requests to `url`, or None for a direct connection.
pub fn for_url(url: &str) -> Option<String> {
    let mut config: WINHTTP_CURRENT_USER_IE_PROXY_CONFIG = unsafe { std::mem::zeroed() };
    if unsafe { WinHttpGetIEProxyConfigForCurrentUser(&mut config) } == 0 {
        return None;
    }
    let script = unsafe { take(config.lpszAutoConfigUrl) };
    let fixed = unsafe { take(config.lpszProxy) };
    let _ = unsafe { take(config.lpszProxyBypass) };
    if config.fAutoDetect != 0 || script.is_some() {
        if let Automatic::Answer(proxy) = automatic(url, script.as_deref()) {
            return proxy;
        }
    }
    fixed.as_deref().and_then(pick)
}

fn automatic(url: &str, script: Option<&str>) -> Automatic {
    let agent = wide("Vorto");
    let session = unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_NO_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    if session.is_null() {
        return Automatic::Failed;
    }
    // Detection on a home network without a proxy must not hold up the download.
    unsafe { WinHttpSetTimeouts(session, 5000, 5000, 5000, 5000) };
    let script = script.map(wide);
    let mut options: WINHTTP_AUTOPROXY_OPTIONS = unsafe { std::mem::zeroed() };
    match &script {
        Some(script) => {
            options.dwFlags = WINHTTP_AUTOPROXY_CONFIG_URL;
            options.lpszAutoConfigUrl = script.as_ptr();
        }
        None => {
            options.dwFlags = WINHTTP_AUTOPROXY_AUTO_DETECT;
            options.dwAutoDetectFlags =
                WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A;
        }
    }
    let url = wide(url);
    let mut info: WINHTTP_PROXY_INFO = unsafe { std::mem::zeroed() };
    // Windows credentials are only offered to the script's server when it asks for them.
    let mut found =
        unsafe { WinHttpGetProxyForUrl(session, url.as_ptr(), &mut options, &mut info) } != 0;
    if !found && unsafe { GetLastError() } == ERROR_WINHTTP_LOGIN_FAILURE {
        options.fAutoLogonIfChallenged = 1;
        found =
            unsafe { WinHttpGetProxyForUrl(session, url.as_ptr(), &mut options, &mut info) } != 0;
    }
    let proxy = unsafe { take(info.lpszProxy) };
    let _ = unsafe { take(info.lpszProxyBypass) };
    unsafe { WinHttpCloseHandle(session) };
    if found {
        Automatic::Answer(proxy.as_deref().and_then(pick))
    } else {
        Automatic::Failed
    }
}

/// Windows lists proxies as `host:port`, or per scheme as `http=host:port;https=host:port`.
fn pick(list: &str) -> Option<String> {
    let entries: Vec<&str> = list
        .split([';', ' '])
        .map(str::trim)
        .filter(|e| {
            !e.is_empty() && !e.eq_ignore_ascii_case("DIRECT") && !e.eq_ignore_ascii_case("PROXY")
        })
        .collect();
    let scheme = |name: &str| entries.iter().find_map(|e| e.strip_prefix(name));
    scheme("https=")
        .or_else(|| entries.iter().copied().find(|e| !e.contains('=')))
        .or_else(|| scheme("http="))
        .map(|p| {
            p.trim_start_matches("http://")
                .trim_start_matches("https://")
                .to_string()
        })
        .filter(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proxy_lists_prefer_the_https_entry() {
        assert_eq!(pick("proxy:8080").as_deref(), Some("proxy:8080"));
        assert_eq!(pick("http=a:80;https=b:443").as_deref(), Some("b:443"));
        assert_eq!(pick("https=https://b:443").as_deref(), Some("b:443"));
        assert_eq!(pick("http=a:80;socks=c:1080").as_deref(), Some("a:80"));
        assert_eq!(pick("PROXY p:3128; DIRECT").as_deref(), Some("p:3128"));
        assert_eq!(pick("DIRECT"), None);
        assert_eq!(pick("socks=c:1080"), None);
        assert_eq!(pick(""), None);
    }
}
