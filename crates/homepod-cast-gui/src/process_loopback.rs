use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use windows::core::{implement, Interface, Ref, Result, HRESULT};
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::Variant::VT_BLOB;

use airplay_audio::{LiveFrameSender, LivePcmFrame};
use crate::controller::{monotonic_now_ms, set_audio_peak_level, DiagnosticCounters, IS_SOURCE_MUTED};

pub const RATE: u32 = 44100;
pub const CHANNELS: u8 = 2;

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct CompletionHandler {
    tx: std::sync::Mutex<Option<std::sync::mpsc::Sender<Result<IAudioClient>>>>,
}

impl CompletionHandler {
    fn new(tx: std::sync::mpsc::Sender<Result<IAudioClient>>) -> Self {
        Self {
            tx: std::sync::Mutex::new(Some(tx)),
        }
    }
}

impl IActivateAudioInterfaceCompletionHandler_Impl for CompletionHandler_Impl {
    fn ActivateCompleted(&self, activateoperation: Ref<'_, IActivateAudioInterfaceAsyncOperation>) -> Result<()> {
        let res = (|| -> Result<IAudioClient> {
            let mut hr = HRESULT(0);
            let mut unk = None;
            unsafe {
                let op = activateoperation.ok()?;
                op.GetActivateResult(&mut hr, &mut unk)?;
                hr.ok()?;
                let unk = unk.ok_or_else(|| windows::core::Error::from(E_FAIL))?;
                unk.cast::<IAudioClient>()
            }
        })();

        if let Ok(mut lock) = self.tx.lock() {
            if let Some(tx) = lock.take() {
                let _ = tx.send(res);
            }
        }
        Ok(())
    }
}

/// Activate an IAudioClient targeted at a specific process tree using Windows 10/11 Process Loopback.
pub fn activate_process_loopback(target_pid: u32) -> Result<IAudioClient> {
    let activation_params = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
            ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                TargetProcessId: target_pid,
                ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
            },
        },
    };

    let blob_size = std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>();
    let p_blob = unsafe { CoTaskMemAlloc(blob_size) as *mut AUDIOCLIENT_ACTIVATION_PARAMS };
    if p_blob.is_null() {
        return Err(windows::core::Error::from(E_OUTOFMEMORY));
    }
    unsafe {
        p_blob.write(activation_params);
    }

    let mut prop_variant: PROPVARIANT = unsafe { std::mem::zeroed() };
    unsafe {
        let inner = &mut prop_variant.Anonymous;
        let anon = &mut *inner.Anonymous;
        anon.vt = VT_BLOB;
        anon.Anonymous.blob.cbSize = blob_size as u32;
        anon.Anonymous.blob.pBlobData = p_blob as *mut u8;
    }

    let (tx, rx) = channel();
    let handler: IActivateAudioInterfaceCompletionHandler = CompletionHandler::new(tx).into();

    unsafe {
        ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID,
            Some(&prop_variant),
            &handler,
        )?;
    }

    rx.recv_timeout(Duration::from_secs(3))
        .map_err(|_| windows::core::Error::from(E_FAIL))?
}

/// A RAII wrapper for an active Process Loopback capture session.
pub struct ProcessLoopbackSession {
    pub audio_client: IAudioClient,
    pub capture_client: IAudioCaptureClient,
    pub event_handle: HANDLE,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

// Safety: Windows COM interface and handles can be transferred across threads in MTA.
unsafe impl Send for ProcessLoopbackSession {}

impl ProcessLoopbackSession {
    pub fn new(target_pid: u32) -> Result<Self> {
        let audio_client = activate_process_loopback(target_pid)?;

        // Note: For VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, GetMixFormat() returns E_NOTIMPL.
        // We supply standard 44.1kHz 16-bit stereo PCM format directly.
        let wf = WAVEFORMATEX {
            wFormatTag: 1, // WAVE_FORMAT_PCM
            nChannels: CHANNELS as u16,
            nSamplesPerSec: RATE,
            wBitsPerSample: 16,
            nBlockAlign: CHANNELS as u16 * 2,
            nAvgBytesPerSec: RATE * (CHANNELS as u32 * 2),
            cbSize: 0,
        };

        info!(
            target_pid,
            sample_rate = RATE,
            channels = CHANNELS,
            bits_per_sample = 16,
            "Initializing process loopback IAudioClient"
        );

        let stream_flags = AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK;

        unsafe {
            audio_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                stream_flags,
                5_000_000, // 500ms reference buffer
                0,
                &wf,
                None,
            )?;
        }

        let event_handle = unsafe { CreateEventW(None, false, false, None)? };
        unsafe {
            audio_client.SetEventHandle(event_handle)?;
        }

        let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService()? };

        Ok(Self {
            audio_client,
            capture_client,
            event_handle,
            sample_rate: RATE,
            channels: CHANNELS as u16,
            bits_per_sample: 16,
        })
    }

    pub fn start(&self) -> Result<()> {
        unsafe { self.audio_client.Start() }
    }

    pub fn stop(&self) -> Result<()> {
        unsafe { self.audio_client.Stop() }
    }
}

impl Drop for ProcessLoopbackSession {
    fn drop(&mut self) {
        let _ = self.stop();
        if !self.event_handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.event_handle);
            }
        }
    }
}

/// Run loopback audio capture on a target application process ID.
/// If `exe_name` is provided, automatically attempts to re-resolve the PID
/// if the process restarts. Continuously pushes converted frames to the `LiveFrameSender`.
pub fn run_process_loopback_capture(
    mut target_pid: u32,
    exe_name: Option<String>,
    sender: LiveFrameSender,
    stop: Arc<AtomicBool>,
    counters: Arc<DiagnosticCounters>,
) -> anyhow::Result<()> {
    let silence = vec![0i16; (RATE as usize / 20) * CHANNELS as usize];
    let bytes_per_frame = (CHANNELS as usize) * 2; // 16-bit stereo = 4 bytes per frame

    while !stop.load(Ordering::Relaxed) {
        // If PID is 0 or invalid, attempt to find by exe_name
        if target_pid == 0 {
            if let Some(ref name) = exe_name {
                let apps = crate::app_enumerator::get_audio_applications();
                if let Some(found) = apps.iter().find(|a| a.exe_name.eq_ignore_ascii_case(name)) {
                    target_pid = found.pid;
                }
            }
        }

        let session = match ProcessLoopbackSession::new(target_pid) {
            Ok(s) => s,
            Err(e) => {
                debug!(target_pid, ?exe_name, "Target process audio not ready ({e}), sending silence and retrying...");
                if let Some(ref name) = exe_name {
                    let apps = crate::app_enumerator::get_audio_applications();
                    if let Some(found) = apps.iter().find(|a| a.exe_name.eq_ignore_ascii_case(name)) {
                        if found.pid != target_pid {
                            info!(old_pid = target_pid, new_pid = found.pid, "Found restarted process for [{name}]");
                            target_pid = found.pid;
                        }
                    }
                }
                let _ = sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        if let Err(e) = session.start() {
            warn!(target_pid, "Failed to start process loopback audio stream: {e}");
            let _ = sender.try_send(LivePcmFrame {
                samples: silence.clone(),
                channels: CHANNELS,
                sample_rate: RATE,
            });
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        info!(target_pid, "Process loopback audio capture active!");

        while !stop.load(Ordering::Relaxed) {
            let wait_res = unsafe { WaitForSingleObject(session.event_handle, 100) };
            if wait_res == WAIT_FAILED {
                warn!("WaitForSingleObject failed on process loopback event");
                break;
            }

            let mut packet_size = match unsafe { session.capture_client.GetNextPacketSize() } {
                Ok(s) => s,
                Err(_) => break, // Process terminated or audio device reset
            };

            while packet_size > 0 && !stop.load(Ordering::Relaxed) {
                let mut p_data: *mut u8 = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                let buf_res = unsafe {
                    session.capture_client.GetBuffer(
                        &mut p_data,
                        &mut num_frames,
                        &mut flags,
                        None,
                        None,
                    )
                };

                if let Ok(()) = buf_res {
                    if num_frames > 0 {
                        counters.frames_captured.fetch_add(num_frames as u64, Ordering::Relaxed);
                        let is_silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;

                        let total_bytes = num_frames as usize * bytes_per_frame;
                        let mut stereo_samples: Vec<i16> = Vec::with_capacity(num_frames as usize * 2);
                        let mut mono_f32: Vec<f32> = Vec::with_capacity(num_frames as usize);
                        let mut max_abs: f32 = 0.0;
                        let mut has_audio = false;

                        if !is_silent && !p_data.is_null() {
                            let raw_slice = unsafe { std::slice::from_raw_parts(p_data, total_bytes) };

                            for frame_chunk in raw_slice.chunks_exact(bytes_per_frame) {
                                let left_i16 = i16::from_le_bytes([frame_chunk[0], frame_chunk[1]]);
                                let right_i16 = i16::from_le_bytes([frame_chunk[2], frame_chunk[3]]);

                                let left_f = left_i16 as f32 / 32768.0;
                                let right_f = right_i16 as f32 / 32768.0;

                                let mono = (left_f + right_f) * 0.5;
                                let abs_m = mono.abs();
                                if abs_m > max_abs {
                                    max_abs = abs_m;
                                }
                                mono_f32.push(mono);

                                if left_i16.abs() > 32 || right_i16.abs() > 32 {
                                    has_audio = true;
                                }
                                stereo_samples.push(left_i16);
                                stereo_samples.push(right_i16);
                            }
                        } else {
                            stereo_samples.resize(num_frames as usize * 2, 0);
                            mono_f32.resize(num_frames as usize, 0.0);
                        }

                        // Feed spectrum and audio peak metrics (respecting global source mute)
                        if IS_SOURCE_MUTED.load(Ordering::Relaxed) {
                            stereo_samples.fill(0);
                            set_audio_peak_level(0.0);
                            crate::spectrum::reset_spectrum_levels();
                        } else {
                            let is_mono = crate::controller::is_audio_mono();
                            let bal = crate::controller::get_audio_balance();
                            if is_mono || bal != 0.0 {
                                for chunk in stereo_samples.chunks_exact_mut(2) {
                                    let mut l = chunk[0] as f32;
                                    let mut r = chunk[1] as f32;
                                    if is_mono {
                                        let m = (l + r) * 0.5;
                                        l = m;
                                        r = m;
                                    }
                                    if bal < 0.0 {
                                        r *= 1.0 + bal;
                                    } else if bal > 0.0 {
                                        l *= 1.0 - bal;
                                    }
                                    chunk[0] = l.clamp(-32768.0, 32767.0) as i16;
                                    chunk[1] = r.clamp(-32768.0, 32767.0) as i16;
                                }
                            }
                            if has_audio {
                                counters.last_active_ts.store(monotonic_now_ms(), Ordering::Relaxed);
                            }
                        }

                        // Send frames directly (already 44.1kHz 16-bit stereo)
                        let _ = sender.try_send(LivePcmFrame {
                            samples: stereo_samples,
                            channels: CHANNELS,
                            sample_rate: RATE,
                        });
                    }

                    unsafe {
                        let _ = session.capture_client.ReleaseBuffer(num_frames);
                    }
                }

                packet_size = unsafe { session.capture_client.GetNextPacketSize().unwrap_or(0) };
            }
        }

        if stop.load(Ordering::Relaxed) {
            break;
        }

        info!(target_pid, "Process loopback session ended or interrupted. Reconnecting in 500ms...");
        let _ = sender.try_send(LivePcmFrame {
            samples: silence.clone(),
            channels: CHANNELS,
            sample_rate: RATE,
        });
        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}
