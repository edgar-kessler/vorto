//! What the engine asks of Windows: processor cores, scheduling, DLL search, crash reports and
//! the Vulkan loader.
use std::ffi::c_void;
use windows_sys::Win32::{
    Storage::FileSystem::WriteFile,
    System::{
        Console::{GetStdHandle, STD_ERROR_HANDLE},
        Diagnostics::Debug::{
            SetErrorMode, SetUnhandledExceptionFilter, EXCEPTION_POINTERS, SEM_FAILCRITICALERRORS,
            SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX,
        },
        LibraryLoader::{
            LoadLibraryExW, SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        },
        SystemInformation::{
            GetLogicalProcessorInformationEx, RelationProcessorCore,
            SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
        },
        Threading::{
            GetCurrentProcess, ProcessPowerThrottling, SetProcessInformation,
            PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            PROCESS_POWER_THROTTLING_STATE,
        },
    },
};

/// The efficiency class of each physical core.
fn core_classes() -> Option<Vec<u8>> {
    type Record = SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
    let mut len = 0u32;
    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, std::ptr::null_mut(), &mut len)
    };
    if len == 0 {
        return None;
    }
    // u64 words keep the records aligned. The extra record's worth of room keeps the last,
    // shorter record inside the allocation when it is read as a full struct.
    let words = (len as usize + std::mem::size_of::<Record>()).div_ceil(8);
    let mut buffer = vec![0u64; words];
    let base = buffer.as_mut_ptr() as *mut Record;
    if unsafe { GetLogicalProcessorInformationEx(RelationProcessorCore, base, &mut len) } == 0 {
        return None;
    }
    let mut classes = Vec::new();
    let mut offset = 0usize;
    while offset < len as usize {
        // Windows fills the buffer with consecutive records of the requested relationship.
        let record = unsafe { &*(base as *const u8).add(offset).cast::<Record>() };
        let size = record.Size as usize;
        if size == 0 || offset + size > len as usize {
            break;
        }
        if record.Relationship == RelationProcessorCore {
            classes.push(unsafe { record.Anonymous.Processor.EfficiencyClass });
        }
        offset += size;
    }
    (!classes.is_empty()).then_some(classes)
}

/// Threads for inference: one per physical core, at most 8. Low-power island cores (the
/// slowest of three or more core types) don't count, and PCs with four cores or fewer keep one
/// free so recording and the desktop stay responsive.
pub fn inference_threads() -> usize {
    let usable = match core_classes() {
        Some(classes) => usable_cores(&classes),
        None => std::thread::available_parallelism()
            .map(|n| n.get() / 2)
            .unwrap_or(4),
    };
    threads_for(usable)
}
fn usable_cores(classes: &[u8]) -> usize {
    let mut kinds = classes.to_vec();
    kinds.sort_unstable();
    kinds.dedup();
    if kinds.len() >= 3 {
        classes.iter().filter(|&&c| c != kinds[0]).count()
    } else {
        classes.len()
    }
}
fn threads_for(cores: usize) -> usize {
    let n = if cores <= 4 {
        cores.saturating_sub(1)
    } else {
        cores
    };
    n.clamp(1, 8)
}

/// DLLs load from the engine's folder and System32 only, never from the current directory or
/// PATH. Delay-loaded libraries such as DirectML and the Vulkan loader follow the same rule.
pub fn safe_dll_search() {
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) };
}

/// Windows 11 slows down background processes to save power, and Vorto's windows are usually
/// hidden while it transcribes. Recognition is short and the user is waiting for it.
pub fn prefer_speed() {
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0,
    };
    // Windows 10 before version 1709 lacks this setting; failing there is harmless.
    unsafe {
        SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &state as *const _ as *const c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
    };
}

/// No Windows error dialogs from this hidden process, and a line in engine.log when it crashes.
pub fn report_crashes() {
    unsafe extern "system" fn crashed(info: *const EXCEPTION_POINTERS) -> i32 {
        // Formats without allocating: the heap may be what failed.
        let code = unsafe {
            info.as_ref()
                .and_then(|i| i.ExceptionRecord.as_ref())
                .map_or(0, |r| r.ExceptionCode as u32)
        };
        let mut line = *b"Voice engine crashed with exception 0x00000000\r\n";
        let digits = b"0123456789ABCDEF";
        for i in 0..8 {
            line[38 + i] = digits[((code >> (28 - 4 * i)) & 0xF) as usize];
        }
        let mut written = 0u32;
        unsafe {
            WriteFile(
                GetStdHandle(STD_ERROR_HANDLE),
                line.as_ptr(),
                line.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };
        // EXCEPTION_CONTINUE_SEARCH: let the process end as it would have.
        0
    }
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
        SetUnhandledExceptionFilter(Some(crashed));
    }
}

/// Whisper's graphics backend delay-loads the Vulkan loader, which comes with graphics drivers.
/// Without it the first Vulkan call would crash the engine instead of failing.
pub fn vulkan_loader_present() -> bool {
    let name: Vec<u16> = "vulkan-1.dll".encode_utf16().chain(Some(0)).collect();
    // Stays loaded: the backend uses it next.
    unsafe { LoadLibraryExW(name.as_ptr(), 0, LOAD_LIBRARY_SEARCH_SYSTEM32) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_processors_keep_a_core_free() {
        assert_eq!(threads_for(1), 1);
        assert_eq!(threads_for(2), 1);
        assert_eq!(threads_for(4), 3);
        assert_eq!(threads_for(6), 6);
        assert_eq!(threads_for(16), 8);
    }
    #[test]
    fn hybrid_processors_use_their_efficiency_cores() {
        // 2 performance + 8 efficiency cores, as in many thin laptops.
        let laptop = [[1u8; 2].as_slice(), &[0; 8]].concat();
        assert_eq!(threads_for(usable_cores(&laptop)), 8);
        // 6 performance + 8 efficiency + 2 low-power island cores.
        let island = [[2u8; 6].as_slice(), &[1; 8], &[0; 2]].concat();
        assert_eq!(usable_cores(&island), 14);
        assert_eq!(usable_cores(&[0; 8]), 8);
    }
    #[test]
    fn this_pc_reports_its_cores() {
        let classes = core_classes().expect("core information");
        assert!(!classes.is_empty());
        assert!(classes.len() <= std::thread::available_parallelism().unwrap().get());
    }
}
