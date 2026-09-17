#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod controller;
mod hotkey;
mod log;
mod native;
mod update;

use std::sync::{mpsc, Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewWindow, WindowEvent,
};

fn tell_controller(app: &AppHandle, msg: controller::Msg) {
    if let Some(handle) = app.try_state::<controller::Handle>() {
        if let Ok(tx) = handle.tx.lock() {
            let _ = tx.send(msg);
        }
    }
}

/// A hidden WebView2 keeps rendering and running timers unless it is told it can't be seen.
/// Suspended, it also frees memory while Vorto sits in the tray.
fn set_awake(window: &WebviewWindow, awake: bool) {
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_3;
        use windows_core::Interface;
        let controller = webview.controller();
        let _ = controller.SetIsVisible(awake);
        if !awake {
            if let Ok(core) = controller
                .CoreWebView2()
                .and_then(|c| c.cast::<ICoreWebView2_3>())
            {
                let done =
                    webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                let _ = core.TrySuspend(&done);
            }
        }
    });
}

/// Tells the controller when the main window appears or disappears, once per change.
fn report_visible(app: &AppHandle, visible: bool) {
    use std::sync::atomic::{AtomicU8, Ordering};
    static LAST: AtomicU8 = AtomicU8::new(0);
    let state = if visible { 2 } else { 1 };
    if LAST.swap(state, Ordering::AcqRel) != state {
        tell_controller(app, controller::Msg::MainWindow { visible });
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        set_awake(&window, true);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        report_visible(app, true);
    }
}

fn hide_main(window: &WebviewWindow) {
    stop_shortcut_capture(window.app_handle());
    let _ = window.hide();
    set_awake(window, false);
    report_visible(window.app_handle(), false);
}

/// Recording a shortcut takes every key, so it must not outlive the window it happens in.
fn stop_shortcut_capture(app: &AppHandle) {
    if hotkey::capturing() {
        tell_controller(
            app,
            controller::Msg::Action(controller::Action::StopRecordingShortcut),
        );
    }
}

/// Launched by Windows at sign-in: stay in the tray.
fn started_hidden() -> bool {
    std::env::args().any(|a| a == "--hidden")
}

/// Asked by the page once it's loaded. A page that stays hidden goes to sleep right away:
/// WebView2 treats a page in a hidden window as visible.
#[tauri::command]
fn start_hidden(window: WebviewWindow) -> bool {
    let hidden = started_hidden();
    if hidden && window.label() == "main" {
        set_awake(&window, false);
        report_visible(window.app_handle(), false);
    }
    hidden
}

/// The page is ready to be seen: show the window unless Vorto started at sign-in.
#[tauri::command]
fn show_window(app: AppHandle) {
    show_main(&app);
}

fn message_box(text: &str, question: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDOK, MB_ICONERROR, MB_OK, MB_OKCANCEL,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let flags = MB_ICONERROR | if question { MB_OKCANCEL } else { MB_OK };
    unsafe { MessageBoxW(0, wide(text).as_ptr(), wide("Vorto").as_ptr(), flags) == IDOK }
}

/// The installer sets up WebView2, but a copied exe or a broken runtime would otherwise fail
/// without a word.
fn webview2_installed() -> bool {
    use webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString;
    use windows_core::{PCWSTR, PWSTR};
    let mut version = PWSTR::null();
    let found =
        unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut version) }
            .is_ok()
            && !version.is_null();
    if !version.is_null() {
        unsafe { windows_sys::Win32::System::Com::CoTaskMemFree(version.0 as _) };
    }
    found
}

fn open_url(url: &str) {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    unsafe {
        ShellExecuteW(
            0,
            wide("open").as_ptr(),
            wide(url).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
}

/// Physical memory in GB, or None when Windows won't say.
fn memory_gb() -> Option<f64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    (unsafe { GlobalMemoryStatusEx(&mut status) } != 0)
        .then(|| status.ullTotalPhys as f64 / 1_073_741_824.0)
}

fn main() {
    if !webview2_installed() {
        if message_box(
            "Vorto needs the Microsoft Edge WebView2 Runtime, which is part of Windows 11 and most Windows 10 PCs.\n\nSelect OK to open Microsoft's download page. Install the Evergreen Runtime, then start Vorto again.",
            true,
        ) {
            open_url("https://developer.microsoft.com/microsoft-edge/webview2/#download");
        }
        return;
    }
    std::panic::set_hook(Box::new(|info| {
        log::write(format!("crashed: {info}"));
        message_box(
            "Vorto ran into a problem and has to close. Start it again to keep dictating.",
            false,
        );
        std::process::exit(1);
    }));
    let hidden = started_hidden();
    let (tx, rx) = mpsc::channel::<controller::Msg>();
    let published = Arc::new(Mutex::new(controller::Published::default()));
    let handle = controller::Handle {
        tx: Mutex::new(tx.clone()),
        published: published.clone(),
    };
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            // Windows starting Vorto at sign-in while it already runs changes nothing.
            if !args.iter().any(|a| a == "--hidden") {
                show_main(app);
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(handle)
        .invoke_handler(tauri::generate_handler![
            controller::get_state,
            controller::action,
            start_hidden,
            show_window
        ])
        .setup(move |app| {
            // Tauri turns an error here into a crash without a word, so it's reported here.
            if let Err(error) = setup(app, tx, rx, published, hidden) {
                log::write(format!("could not start: {error}"));
                message_box(&format!("Vorto couldn't start.\n\n{error}"), false);
                std::process::exit(1);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let Some(main) = window.app_handle().get_webview_window("main") else {
                return;
            };
            match event {
                // Closing keeps Vorto in the tray so the shortcut keeps working.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    hide_main(&main);
                }
                WindowEvent::Focused(false) => stop_shortcut_capture(window.app_handle()),
                // Minimizing and restoring arrive as size changes.
                WindowEvent::Resized(_) => {
                    let visible =
                        main.is_visible().unwrap_or(false) && !main.is_minimized().unwrap_or(false);
                    report_visible(window.app_handle(), visible);
                }
                _ => {}
            }
        });
    if let Err(error) = result.run(tauri::generate_context!()) {
        log::write(format!("could not start: {error}"));
        message_box(&format!("Vorto couldn't start.\n\n{error}"), false);
    }
}

fn setup(
    app: &mut tauri::App,
    tx: mpsc::Sender<controller::Msg>,
    rx: mpsc::Receiver<controller::Msg>,
    published: Arc<Mutex<controller::Published>>,
    hidden: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = vorto::data::Store::open()?;
    log::init(&store.root);
    log::write(format!("Vorto {} starting", env!("CARGO_PKG_VERSION")));
    if store.fresh && memory_gb().is_some_and(|gb| gb <= 8.5) {
        // On PCs with 8 GB or less, the model leaves memory after 15 idle minutes.
        store.settings.idle_minutes = 15;
    }
    if store.settings.autostart {
        native::restore_autostart();
    }

    let (hook_tx, hook_rx) = mpsc::channel();
    hotkey::configure(&store.settings.hotkey, store.settings.toggle);
    hotkey::start(hook_tx);
    let forward = tx.clone();
    std::thread::spawn(move || {
        for event in hook_rx {
            log::write(format!("hook event: {event:?}"));
            if forward.send(controller::Msg::Hook(event)).is_err() {
                break;
            }
        }
    });

    if let Some(main) = app.get_webview_window("main") {
        // Match the first painted frame to the theme so opening never flashes.
        let dark = matches!(main.theme(), Ok(tauri::Theme::Dark));
        let color = if dark { (17, 17, 19) } else { (251, 251, 251) };
        let _ =
            main.set_background_color(Some(tauri::window::Color(color.0, color.1, color.2, 255)));
    }
    let hud = app
        .get_webview_window("hud")
        .ok_or("The recording indicator window is missing.")?;
    let hud_hwnd = hud.hwnd()?.0 as isize;
    // tao rewrites the extended styles when it changes window flags, so this runs first.
    let _ = hud.set_ignore_cursor_events(true);
    native::prepare_overlay(hud_hwnd);
    let hud_size = hud.outer_size()?;
    let hud_size = (hud_size.width as i32, hud_size.height as i32);

    let open = MenuItem::with_id(app, "open", "Open Vorto", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Vorto", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("vorto")
        .tooltip("Vorto")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }
    tray.build(app)?;

    let controller = controller::Controller::new(
        app.handle().clone(),
        tx,
        published,
        store,
        (hud_hwnd, hud_size),
        hidden,
    );
    std::thread::Builder::new()
        .name("vorto-controller".into())
        .spawn(move || controller.run(rx))?;
    Ok(())
}
