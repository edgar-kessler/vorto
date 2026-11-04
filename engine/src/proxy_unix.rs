//! The proxy for a download, from the environment as is usual on macOS and Linux.
pub fn for_url(_url: &str) -> Option<String> {
    ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
}
