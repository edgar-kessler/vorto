//! What the engine asks of macOS and Linux: only how many threads to use.

/// One thread per core, at most 8, keeping one free for recording on smaller machines.
pub fn inference_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    (if cores <= 4 {
        cores.saturating_sub(1)
    } else {
        cores
    })
    .clamp(1, 8)
}
pub fn safe_dll_search() {}
pub fn prefer_speed() {}
pub fn report_crashes() {}
pub fn vulkan_loader_present() -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn at_least_one_thread() {
        assert!(super::inference_threads() >= 1);
    }
}
