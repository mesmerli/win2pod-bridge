//! Test tool to list all detectable audio applications and windowed programs.
//! Usage: cargo run --example test_app_enumerator

use wasapi::initialize_mta;
use homepod_cast_gui::app_enumerator::get_audio_applications;

fn main() {
    let _com = initialize_mta();

    println!("============================================================");
    println!("Scanning system for Audio Applications & GUI Programs...");
    println!("============================================================");

    let apps = get_audio_applications();

    println!("\nFound {} applications:", apps.len());
    println!("{:<8} | {:<20} | {:<10} | {}", "PID", "Executable", "Status", "Display Title");
    println!("{}", "-".repeat(75));

    for app in apps {
        let status = if app.is_active_audio {
            "🔊 PLAYING"
        } else {
            "⚪ Inactive"
        };
        println!("{:<8} | {:<20} | {:<10} | {}", app.pid, app.exe_name, status, app.display_name);
    }

    println!("============================================================");
}
