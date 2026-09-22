//! Where text just landed on screen, for the short glow over inserted words. Asks the app
//! through UI Automation for the rectangles of the last `chars` characters before the caret,
//! and falls back to the system caret that classic Win32 controls show. Reads positions only:
//! nothing in the app is selected or changed.
use windows::{
    core::{Interface, BOOL},
    Win32::{
        Foundation::{HWND, POINT},
        Graphics::Gdi::ClientToScreen,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED,
            },
            Ole::{
                SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetUBound, SafeArrayUnaccessData,
            },
        },
        UI::{
            Accessibility::{
                CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationTextPattern,
                IUIAutomationTextPattern2, IUIAutomationTextRange, TextPatternRangeEndpoint_Start,
                TextUnit_Character, UIA_TextPattern2Id, UIA_TextPatternId,
            },
            WindowsAndMessaging::{GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO},
        },
    },
};

/// A rectangle on screen in physical pixels: left, top, width, height.
pub type Rect = (i32, i32, i32, i32);

/// Rectangles of the inserted text, one per line, or of the caret. Empty when the app
/// doesn't tell.
pub fn inserted(hwnd: isize, chars: usize) -> Vec<Rect> {
    // SAFETY: COM is set up and torn down on this thread; every pointer comes from Windows.
    let mut rects = unsafe { from_automation(chars) }.unwrap_or_default();
    if rects.is_empty() {
        // Chromium, and with it most browsers and Electron apps, turns on its UI Automation
        // support when first asked and answers the next question.
        std::thread::sleep(std::time::Duration::from_millis(250));
        rects = unsafe { from_automation(chars) }.unwrap_or_default();
    }
    if !rects.is_empty() {
        return rects;
    }
    system_caret(hwnd).into_iter().collect()
}

unsafe fn from_automation(chars: usize) -> windows::core::Result<Vec<Rect>> {
    let initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
    let result = (|| {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        // A busy or hung app must not keep the worker waiting.
        if let Ok(timeouts) = automation.cast::<IUIAutomation2>() {
            let _ = timeouts.SetConnectionTimeout(800);
            let _ = timeouts.SetTransactionTimeout(800);
        }
        let element = automation.GetFocusedElement()?;
        let caret: IUIAutomationTextRange =
            match element.GetCurrentPatternAs::<IUIAutomationTextPattern2>(UIA_TextPattern2Id) {
                Ok(pattern) => {
                    let mut active = BOOL(0);
                    pattern.GetCaretRange(&mut active)?
                }
                Err(_) => {
                    let pattern = element
                        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)?;
                    pattern.GetSelection()?.GetElement(0)?
                }
            };
        let count = chars.clamp(1, 4000) as i32;
        caret.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -count)?;
        rectangles(&caret)
    })();
    if initialized {
        CoUninitialize();
    }
    result
}

/// The bounding rectangles of a text range: a flat array of left, top, width, height.
unsafe fn rectangles(range: &IUIAutomationTextRange) -> windows::core::Result<Vec<Rect>> {
    let array = range.GetBoundingRectangles()?;
    if array.is_null() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let upper = SafeArrayGetUBound(array, 1).unwrap_or(-1);
    let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
    if upper >= 3 && SafeArrayAccessData(array, &mut data).is_ok() {
        let values = std::slice::from_raw_parts(data as *const f64, upper as usize + 1);
        for &[x, y, w, h] in values.as_chunks::<4>().0 {
            if w > 0.0 && h > 0.0 {
                out.push((x as i32, y as i32, w.ceil() as i32, h.ceil() as i32));
            }
        }
        let _ = SafeArrayUnaccessData(array);
    }
    let _ = SafeArrayDestroy(array);
    Ok(out)
}

/// The caret of classic Win32 edit controls, such as Notepad's.
fn system_caret(hwnd: isize) -> Option<Rect> {
    unsafe {
        let thread = GetWindowThreadProcessId(HWND(hwnd as _), None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(thread, &mut info).ok()?;
        if info.hwndCaret.is_invalid() {
            return None;
        }
        let r = info.rcCaret;
        let mut corner = POINT {
            x: r.left,
            y: r.top,
        };
        if !ClientToScreen(info.hwndCaret, &mut corner).as_bool() {
            return None;
        }
        let height = r.bottom - r.top;
        (height > 0).then_some((corner.x, corner.y, (r.right - r.left).max(2), height))
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::{System::Threading::AttachThreadInput, UI::WindowsAndMessaging::*};

    /// Needs a desktop: opens a window with a text box, fills it and asks where the text is.
    /// `cargo test -p vorto-app -- --ignored caret --nocapture`
    #[test]
    #[ignore]
    fn caret_finds_inserted_text() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || unsafe {
            let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
            let edit = CreateWindowExW(
                0,
                wide("EDIT").as_ptr(),
                wide("").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE | 0x0004, // ES_MULTILINE
                200,
                200,
                700,
                240,
                0,
                0,
                0,
                std::ptr::null(),
            );
            let text = wide("Hello from Vorto, this line should glow.");
            SendMessageW(edit, WM_SETTEXT, 0, text.as_ptr() as isize);
            SendMessageW(edit, 0x00B1, 40, 40); // EM_SETSEL: caret after the text
                                                // The way Vorto brings a window forward before inserting.
            use windows_sys::Win32::System::Threading::GetCurrentThreadId;
            let me = GetCurrentThreadId();
            let front = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
            AttachThreadInput(me, front, 1);
            SetForegroundWindow(edit);
            BringWindowToTop(edit);
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(edit);
            AttachThreadInput(me, front, 0);
            tx.send(edit).unwrap();
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
        let edit = rx.recv().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let automation = unsafe { super::from_automation(40) };
        let caret = super::system_caret(edit);
        println!("automation: {automation:?}");
        println!("system caret: {caret:?}");
        unsafe { PostMessageW(edit, WM_CLOSE, 0, 0) };
        let lines = automation.unwrap_or_default();
        assert!(caret.is_some() || !lines.is_empty());
    }

    /// Needs Microsoft Edge: a text box in Chromium, the engine of most browsers and Electron apps.
    #[test]
    #[ignore]
    fn caret_finds_text_in_chromium() {
        let page = "data:text/html,<title>VortoCaretTest</title><textarea id=t rows=3 cols=60>Hello from Vorto, this line should glow.</textarea><script>t.focus();t.setSelectionRange(99,99)</script>";
        std::process::Command::new("cmd")
            .args(["/c", "start", "msedge", "--new-window", page])
            .status()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(4));
        let mut found = 0isize;
        unsafe extern "system" fn each(hwnd: isize, found: isize) -> i32 {
            let title = crate::native::window_title(hwnd);
            if title.contains("VortoCaretTest") {
                *(found as *mut isize) = hwnd;
                return 0;
            }
            1
        }
        unsafe { EnumWindows(Some(each), &mut found as *mut isize as isize) };
        assert!(found != 0, "the Edge window opened");
        unsafe {
            use windows_sys::Win32::System::Threading::GetCurrentThreadId;
            let me = GetCurrentThreadId();
            let front = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
            AttachThreadInput(me, front, 1);
            SetForegroundWindow(found);
            BringWindowToTop(found);
            AttachThreadInput(me, front, 0);
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
        let lines = super::inserted(found, 40);
        println!("chromium: {lines:?}");
        unsafe { PostMessageW(found, WM_CLOSE, 0, 0) };
        assert!(lines.iter().any(|r| r.2 > 100), "the whole line is covered");
    }
}
