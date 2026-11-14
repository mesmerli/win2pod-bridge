//! Process Loopback verification tool
//! Usage:
//!   cargo run --example test_process_loopback
//!   cargo run --example test_process_loopback -- <PID>

use std::time::{Duration, Instant};
use wasapi::initialize_mta;
use windows::core::Interface;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;

use homepod_cast_gui::process_loopback::ProcessLoopbackSession;

fn main() -> anyhow::Result<()> {
    let _com = initialize_mta();

    let args: Vec<String> = std::env::args().collect();
    let target_pid: u32 = if args.len() > 1 {
        args[1].parse().expect("Please provide a valid numeric PID")
    } else {
        println!("Scanning for active audio sessions...");
        match find_active_audio_pid() {
            Some((pid, name)) => {
                println!("Found active audio session: [{name}] (PID: {pid})");
                pid
            }
            None => {
                let current_pid = std::process::id();
                println!("No active audio sessions found. Testing self PID: {current_pid}");
                current_pid
            }
        }
    };

    println!("\n=== Starting Process Loopback Capture on PID: {target_pid} ===");

    let session = match ProcessLoopbackSession::new(target_pid) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to activate Process Loopback for PID {target_pid}: {e}");
            eprintln!("Note: Ensure target application is running and your Windows build is Windows 10 (21H2+) or Windows 11.");
            return Ok(());
        }
    };

    println!("Audio Client initialized successfully:");
    println!("  Sample Rate:     {} Hz", session.sample_rate);
    println!("  Channels:        {}", session.channels);
    println!("  Bits per sample: {}", session.bits_per_sample);

    session.start()?;
    println!("Audio stream started. Capturing audio for 5 seconds...\n");

    let start = Instant::now();
    let mut total_packets = 0u64;
    let mut total_frames = 0u64;
    let bytes_per_frame = (session.channels * (session.bits_per_sample / 8)) as usize;

    while start.elapsed() < Duration::from_secs(5) {
        let wait_res = unsafe { WaitForSingleObject(session.event_handle, 100) };
        if wait_res == WAIT_FAILED {
            eprintln!("Wait failed");
            break;
        }

        let mut packet_size = unsafe { session.capture_client.GetNextPacketSize().unwrap_or(0) };
        while packet_size > 0 {
            let mut p_data: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            let res = unsafe {
                session.capture_client.GetBuffer(
                    &mut p_data,
                    &mut num_frames,
                    &mut flags,
                    None,
                    None,
                )
            };

            if res.is_ok() && num_frames > 0 {
                total_packets += 1;
                total_frames += num_frames as u64;

                let is_silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;
                let mut max_abs = 0.0f32;

                if !is_silent && !p_data.is_null() && session.bits_per_sample == 16 {
                    let total_bytes = num_frames as usize * bytes_per_frame;
                    let raw_slice = unsafe { std::slice::from_raw_parts(p_data, total_bytes) };
                    for b in raw_slice.chunks_exact(2) {
                        let sample_i16 = i16::from_le_bytes([b[0], b[1]]);
                        let abs_f = (sample_i16.abs() as f32) / 32768.0;
                        if abs_f > max_abs {
                            max_abs = abs_f;
                        }
                    }
                }

                // Render ASCII VU meter
                let bar_len = (max_abs * 30.0).clamp(0.0, 30.0) as usize;
                let bar: String = "=".repeat(bar_len);
                let spaces: String = " ".repeat(30 - bar_len);
                print!(
                    "\r[Packet #{:04} | Frames: {:04}] Level: [{}{}] Peak: {:.4} {}",
                    total_packets,
                    num_frames,
                    bar,
                    spaces,
                    max_abs,
                    if is_silent { "(silent)" } else { "" }
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());

                unsafe {
                    let _ = session.capture_client.ReleaseBuffer(num_frames);
                }
            }

            packet_size = unsafe { session.capture_client.GetNextPacketSize().unwrap_or(0) };
        }
    }

    session.stop()?;
    println!("\n\nCapture test completed successfully!");
    println!("Total packets captured: {total_packets}");
    println!("Total frames captured:  {total_frames}");

    Ok(())
}

/// Helper to scan for active audio sessions and return (PID, friendly_name)
fn find_active_audio_pid() -> Option<(u32, String)> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let default_device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()?;
        let session_manager: IAudioSessionManager2 =
            default_device.Activate(CLSCTX_ALL, None).ok()?;
        let session_enumerator = session_manager.GetSessionEnumerator().ok()?;
        let count = session_enumerator.GetCount().ok()?;

        let current_pid = std::process::id();

        for i in 0..count {
            if let Ok(session) = session_enumerator.GetSession(i) {
                if let Ok(session2) = session.cast::<IAudioSessionControl2>() {
                    if let Ok(pid) = session2.GetProcessId() {
                        if pid != 0 && pid != current_pid {
                            let is_system = session2.IsSystemSoundsSession() == S_OK;
                            if !is_system {
                                let name = get_process_name(pid).unwrap_or_else(|| format!("PID: {pid}"));
                                return Some((pid, name));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_process_name(pid: u32) -> Option<String> {
    use windows::Win32::System::Threading::*;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 260];
        let mut size = buffer.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if res.is_ok() && size > 0 {
            let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
            let file_name = std::path::Path::new(&full_path)
                .file_name()?
                .to_string_lossy()
                .to_string();
            return Some(file_name);
        }
    }
    None
}
