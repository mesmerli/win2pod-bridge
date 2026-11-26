//! Main entry point for HomePod-Cast GUI (AirFlash-style pure Rust UI).

// Hide console window in Windows subsystem
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use homepod_cast_gui::{app_lifecycle, controller, gui, single_instance, snapping, tray};

use std::sync::{Arc, RwLock};
use eframe::egui;

use crate::controller::{spawn_controller, ControllerState, GuiCmd};
use crate::gui::FlyoutApp;
use crate::tray::spawn_tray_thread;

fn main() -> anyhow::Result<()> {
    let log_level = if let Ok(lvl) = std::env::var("WIN2POD_LOG").or_else(|_| std::env::var("HOMEPOD_CAST_LOG")) {
        match lvl.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            _ => tracing::Level::ERROR,
        }
    } else {
        #[cfg(debug_assertions)]
        {
            tracing::Level::DEBUG
        }
        #[cfg(not(debug_assertions))]
        {
            // In release mode, log INFO level for diagnostics (debug.log rotates at 1MB)
            tracing::Level::INFO
        }
    };

    let log_dir = crate::controller::app_data_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("debug.log");

    // Prevent infinite log growth: if debug.log exceeds 1MB on startup, rotate it to debug.log.old
    if let Ok(meta) = std::fs::metadata(&log_file) {
        if meta.len() > 1_000_000 {
            let old_log = log_dir.join("debug.log.old");
            let _ = std::fs::rename(&log_file, &old_log);
        }
    }

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let subscriber = tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(file))
            .with_max_level(log_level)
            .with_ansi(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        tracing_subscriber::fmt::init();
    }

    tracing::info!("============================================================");
    tracing::info!("Win2Pod Bridge application starting up at {:?}", std::time::SystemTime::now());

    // Set panic hook to log any panic details to debug.log
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unspecified payload"
        };
        tracing::error!("FATAL PANIC at {location}: {payload}");
        eprintln!("FATAL PANIC at {location}: {payload}");
        crate::controller::restore_original_speaker_mute();
        crate::controller::restore_initial_default_device();
    }));

    // Parse command line arguments:
    // When launched on Windows startup (HKCU Run key) or with minimized flags,
    // we launch silently to the system tray without showing the flyout window.
    let args: Vec<String> = std::env::args().collect();
    let start_minimized = args.iter().any(|a| {
        a == "--startup" || a == "--minimized" || a == "--tray" || a == "-m" || a == "/startup" || a == "/minimized"
    });
    if start_minimized {
        tracing::info!("Launching in background/minimized mode (--startup / --minimized mode)");
    }

    // Check single instance: if another instance is already running:
    // If running in startup/minimized mode, do NOT wake the existing instance's window; just exit quietly.
    // If running normally (user clicked shortcut), wake and show the existing instance.
    if !crate::single_instance::check_or_wake_existing(!start_minimized)? {
        return Ok(());
    }

    // Capture initial default playback device before any audio routing or shields take effect
    crate::controller::capture_initial_default_device();

    // Initialize global application lifecycle finite state machine
    app_lifecycle::init_app_lifecycle(start_minimized);

    // Register atexit handler so original speaker mute and default device are restored on any exit
    crate::controller::register_exit_handler();


    // Register Post-DSP tap for real-time spectrum and output peak monitoring
    airplay_audio::set_post_dsp_hook(Some(Arc::new(|samples| {
        homepod_cast_gui::spectrum::process_post_dsp_pcm(samples);
    })));

    // Async channel for GUI & Tray -> Controller commands
    let (cmd_tx, cmd_rx) = async_channel::unbounded::<GuiCmd>();

    static CONSOLE_CMD_TX: std::sync::OnceLock<async_channel::Sender<GuiCmd>> = std::sync::OnceLock::new();
    let _ = CONSOLE_CMD_TX.set(cmd_tx.clone());

    #[cfg(windows)]
    unsafe {
        type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;
        extern "system" {
            fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
        }
        unsafe extern "system" fn ctrl_handler(_: u32) -> i32 {
            if let Some(tx) = CONSOLE_CMD_TX.get() {
                crate::tray::trigger_app_quit(tx);
                std::thread::sleep(std::time::Duration::from_millis(2000));
            }
            1
        }
        SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    }

    // Shared thread-safe state (RwLock) - prevents memory accumulation when GUI is minimized/hidden
    let shared_state = Arc::new(RwLock::new(ControllerState::default()));

    // Spawn background Tokio controller
    spawn_controller(cmd_tx.clone(), cmd_rx, shared_state.clone());

    let app_cmd_tx = cmd_tx.clone();
    let tray_cmd_tx = cmd_tx.clone();
    let app_shared_state = shared_state.clone();
    let tray_shared_state = shared_state;

    // Load custom window and taskbar icon
    let icon_data = load_window_icon();
    snapping::init_window_pinned_states();
    let initial_always_on_top = crate::controller::load_always_on_top();
    let initial_level = if initial_always_on_top {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    };
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_title("Win2Pod Bridge")
        .with_inner_size([460.0, 680.0])
        .with_min_inner_size([380.0, 500.0])
        .with_resizable(true)
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(!start_minimized)
        .with_window_level(initial_level);

    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(std::sync::Arc::new(icon));
    }

    // Window options for modern Flyout
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        multisampling: 0,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    tracing::info!("Launching eframe::run_native...");

    let res = eframe::run_native(
        "Win2Pod Bridge",
        native_options,
        Box::new(move |cc| {
            let egui_ctx = cc.egui_ctx.clone();
            // Listen for wake signals from any duplicate launches
            crate::single_instance::spawn_wake_listener(egui_ctx.clone());

            // Spawn dedicated background thread for System Tray with Win32 message pump
            spawn_tray_thread(tray_cmd_tx, tray_shared_state, egui_ctx);

            Ok(Box::new(FlyoutApp::new(cc, app_cmd_tx, app_shared_state, start_minimized)))
        }),
    );

    tracing::info!("eframe::run_native finished with result: {:?}", res.as_ref().map(|_| ()));
    res.map_err(|e| anyhow::anyhow!("eframe error: {e:?}"))?;

    let _ = cmd_tx.try_send(GuiCmd::Quit);
    std::thread::sleep(std::time::Duration::from_millis(500));
    homepod_cast_gui::controller::restore_original_speaker_state();
    homepod_cast_gui::controller::restore_initial_default_device();
    Ok(())
}

fn load_window_icon() -> Option<egui::IconData> {
    const ICON_BYTES: &[u8] = include_bytes!("../assets/app_icon.png");
    match image::load_from_memory(ICON_BYTES) {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let (width, height) = rgba.dimensions();
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            })
        }
        Err(e) => {
            tracing::warn!("Failed to load window icon: {e}");
            None
        }
    }
}
