//! Main entry point for HomePod-Cast GUI (AirFlash-style pure Rust UI).

// Hide console window in Windows subsystem
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod autostart;
mod controller;
mod gui;
pub mod i18n;
pub mod license;
mod single_instance;
mod tray;

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
            // Release mode: only record critical errors to prevent infinite log growth
            tracing::Level::ERROR
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
        .write(true)
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

    // Register atexit handler so original speaker mute is restored on any exit
    crate::controller::register_exit_handler();

    // Async channel for GUI & Tray -> Controller commands
    let (cmd_tx, cmd_rx) = async_channel::unbounded::<GuiCmd>();

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
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_title("Win2Pod Bridge")
        .with_inner_size([420.0, 440.0])
        .with_min_inner_size([360.0, 320.0])
        .with_resizable(true)
        .with_close_button(true)
        .with_visible(!start_minimized);

    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(std::sync::Arc::new(icon));
    }

    // Window options for modern Flyout
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
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

    crate::controller::restore_original_speaker_mute();
    let _ = cmd_tx.send_blocking(GuiCmd::Quit);
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
