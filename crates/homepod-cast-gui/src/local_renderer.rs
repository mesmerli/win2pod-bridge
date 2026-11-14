//! Local Audio Renderer (WASAPI Render) & AO (Audio Output / Audio Only) Processing Engine
//!
//! Provides:
//! 1. `LocalAudioRenderer`: Low-latency WASAPI shared render client with hardware/OS auto-conversion.
//! 2. `run_ao_process_engine`: Complete background loop capturing target process audio,
//!    running it through Win2Pod DSP (10-band EQ, Enhancer, Warmth), driving real-time
//!    spectrum visualizers (Paint Pulse), and rendering enhanced audio to local speakers/headphones.
//! 3. `run_ao_system_engine`: Captures system audio loopback to drive DSP & Paint Pulse visualizer
//!    even when no HomePod is connected.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use airplay_audio::eq::{EqConfig, EqParams, Equalizer};
use airplay_audio::enhancer::{SoundEnhancer, SoundEnhancerParams};
use airplay_audio::warmth::{WarmthEnhancer, WarmthEnhancerParams};

use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;

use crate::process_loopback::ProcessLoopbackSession;

const RATE: u32 = 44100;
const CHANNELS: u16 = 2;

/// WASAPI Shared Audio Renderer
pub struct LocalAudioRenderer {
    client: IAudioClient,
    render_client: IAudioRenderClient,
    buffer_frame_count: u32,
    channels: u16,
}

impl LocalAudioRenderer {
    pub fn new(device_id: Option<&str>) -> anyhow::Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = match device_id {
                Some(id) if !id.is_empty() && id != "default" => {
                    let hstring = windows::core::HSTRING::from(id);
                    enumerator.GetDevice(&hstring)?
                }
                _ => enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?,
            };
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

            let sample_rate = RATE;
            let channels = CHANNELS;
            let bits_per_sample = 16u16;
            let block_align = channels * (bits_per_sample / 8);
            let avg_bytes_per_sec = sample_rate * block_align as u32;

            let wfx = WAVEFORMATEX {
                wFormatTag: 1, // WAVE_FORMAT_PCM
                nChannels: channels,
                nSamplesPerSec: sample_rate,
                nAvgBytesPerSec: avg_bytes_per_sec,
                nBlockAlign: block_align,
                wBitsPerSample: bits_per_sample,
                cbSize: 0,
            };

            // AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM (0x80000000) | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY (0x08000000)
            let stream_flags = 0x80000000 | 0x08000000;
            let hns_buffer_duration: i64 = 600_000; // 60ms buffer

            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                stream_flags,
                hns_buffer_duration,
                0,
                &wfx as *const _ as *const _,
                None,
            )?;

            let buffer_frame_count = client.GetBufferSize()?;
            let render_client: IAudioRenderClient = client.GetService()?;

            client.Start()?;

            Ok(Self {
                client,
                render_client,
                buffer_frame_count,
                channels,
            })
        }
    }

    pub fn write_frames(&self, pcm_samples: &[i16]) -> anyhow::Result<usize> {
        let frames_to_write = (pcm_samples.len() / self.channels as usize) as u32;
        if frames_to_write == 0 {
            return Ok(0);
        }

        unsafe {
            let padding = self.client.GetCurrentPadding()?;
            let available = self.buffer_frame_count.saturating_sub(padding);
            let num_frames = frames_to_write.min(available);

            if num_frames == 0 {
                return Ok(0);
            }

            let p_data = self.render_client.GetBuffer(num_frames)?;
            let bytes_to_copy = num_frames as usize * self.channels as usize * 2;
            let src_bytes = std::slice::from_raw_parts(
                pcm_samples.as_ptr() as *const u8,
                bytes_to_copy,
            );
            std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), p_data, bytes_to_copy);

            self.render_client.ReleaseBuffer(num_frames, 0)?;
            Ok(num_frames as usize)
        }
    }
}

impl Drop for LocalAudioRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.Stop();
        }
    }
}

/// Runs the continuous AO (Audio Output) processing loop for an application:
/// - Captures unattenuated audio from `target_pid` via Process Loopback
/// - Passes audio through Win2Pod DSP (10-band EQ, Enhancer, Warmth)
/// - Feeds real-time spectrum analyzer for Paint Pulse visualizer
/// - Renders enhanced audio to local speakers/headphones
pub fn run_ao_process_engine(
    mut target_pid: u32,
    exe_name: Option<String>,
    output_device_id: Option<String>,
    enable_render: bool,
    stop: Arc<AtomicBool>,
    eq_params: Arc<EqParams>,
    enhancer_params: Arc<SoundEnhancerParams>,
    warmth_params: Arc<WarmthEnhancerParams>,
) -> anyhow::Result<()> {
    let _com = wasapi::initialize_mta();

    // Proactively refresh target_pid if executable name is known
    if let Some(ref name) = exe_name {
        if let Some(fresh_pid) = crate::app_enumerator::find_pid_by_exe(name) {
            target_pid = fresh_pid;
        }
    }

    tracing::info!(
        target_pid,
        ?exe_name,
        ?output_device_id,
        enable_render,
        "Starting AO (Process Loopback) engine"
    );

    let mut renderer = if enable_render {
        match LocalAudioRenderer::new(output_device_id.as_deref()) {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::warn!("Failed to initialize LocalAudioRenderer ({e}), will retry in background");
                None
            }
        }
    } else {
        None
    };

    let mut eq = Equalizer::new(EqConfig::ten_band(), eq_params, RATE);
    let mut enhancer = SoundEnhancer::new(enhancer_params, RATE);
    let mut warmth = WarmthEnhancer::new(warmth_params, RATE);

    let bytes_per_frame = (CHANNELS as usize) * 2;

    while !stop.load(Ordering::Relaxed) {
        if target_pid == 0 {
            if let Some(ref name) = exe_name {
                if let Some(fresh_pid) = crate::app_enumerator::find_pid_by_exe(name) {
                    target_pid = fresh_pid;
                }
            }
        }

        // Ensure renderer is initialized / restored if previously invalidated
        if enable_render && renderer.is_none() {
            if let Ok(r) = LocalAudioRenderer::new(output_device_id.as_deref()) {
                renderer = Some(r);
            }
        }

        let session = match ProcessLoopbackSession::new(target_pid) {
            Ok(s) => s,
            Err(_) => {
                if let Some(ref name) = exe_name {
                    if let Some(fresh_pid) = crate::app_enumerator::find_pid_by_exe(name) {
                        if fresh_pid != target_pid {
                            target_pid = fresh_pid;
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(300));
                continue;
            }
        };

        if let Err(e) = session.start() {
            tracing::warn!(target_pid, "Failed to start AO loopback stream: {e}");
            std::thread::sleep(Duration::from_millis(300));
            continue;
        }

        tracing::info!(target_pid, "AO loopback capture active! Streaming through DSP & visualizer.");

        while !stop.load(Ordering::Relaxed) {
            if crate::controller::DEFAULT_DEVICE_CHANGED.swap(false, Ordering::SeqCst) {
                tracing::info!("Audio device change signaled, recycling process AO capture...");
                renderer = None;
                break;
            }

            let wait_res = unsafe { WaitForSingleObject(session.event_handle, 60) };
            if wait_res == WAIT_FAILED {
                tracing::warn!("WaitForSingleObject failed on AO loopback event, recycling...");
                break;
            }

            let mut packet_size = match unsafe { session.capture_client.GetNextPacketSize() } {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("AO capture client error: {e}, recycling capture session...");
                    break;
                }
            };

            while packet_size > 0 && !stop.load(Ordering::Relaxed) {
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

                if let Err(e) = res {
                    tracing::warn!("AO capture GetBuffer error: {e}, recycling session...");
                    break;
                }

                if num_frames > 0 {
                    let is_silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;
                    let total_bytes = num_frames as usize * bytes_per_frame;

                    let mut samples: Vec<i16> = Vec::with_capacity(num_frames as usize * 2);
                    if !is_silent && !p_data.is_null() {
                        let raw_slice = unsafe { std::slice::from_raw_parts(p_data, total_bytes) };
                        for frame_chunk in raw_slice.chunks_exact(bytes_per_frame) {
                            let left = i16::from_le_bytes([frame_chunk[0], frame_chunk[1]]);
                            let right = i16::from_le_bytes([frame_chunk[2], frame_chunk[3]]);
                            samples.push(left);
                            samples.push(right);
                        }
                    } else {
                        samples.resize(num_frames as usize * 2, 0);
                    }

                    // 1. DSP Processing Pipeline
                    enhancer.process(&mut samples);
                    eq.process(&mut samples);
                    warmth.process(&mut samples);

                    // 1b. Balance and Mono Mix
                    let is_mono = crate::controller::is_audio_mono();
                    let bal = crate::controller::get_audio_balance();
                    if is_mono || bal != 0.0 {
                        for chunk in samples.chunks_exact_mut(2) {
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

                    // 2. Feed Spectrum & VU meters for Paint Pulse visualizer
                    crate::spectrum::process_post_dsp_pcm(&samples);

                    // 3. Render directly to local speakers (if enabled)
                    if let Some(ref r) = renderer {
                        let mut offset = 0;
                        let mut retries = 0;
                        let mut write_failed = false;
                        while offset < samples.len() && retries < 3 {
                            match r.write_frames(&samples[offset..]) {
                                Ok(written_frames) if written_frames > 0 => {
                                    offset += written_frames * CHANNELS as usize;
                                    retries = 0;
                                }
                                Ok(_) => {
                                    retries += 1;
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                                Err(e) => {
                                    tracing::warn!("LocalAudioRenderer write error: {e}");
                                    write_failed = true;
                                    break;
                                }
                            }
                        }
                        if write_failed {
                            renderer = None;
                            break;
                        }
                    }

                    unsafe {
                        let _ = session.capture_client.ReleaseBuffer(num_frames);
                    }
                }

                packet_size = match unsafe { session.capture_client.GetNextPacketSize() } {
                    Ok(s) => s,
                    Err(_) => break,
                };
            }
        }

        crate::spectrum::reset_spectrum_levels();
        crate::controller::set_audio_peak_level(0.0);
        std::thread::sleep(Duration::from_millis(300));

        if stop.load(Ordering::Relaxed) {
            break;
        }
    }

    crate::spectrum::reset_spectrum_levels();
    crate::controller::set_audio_peak_level(0.0);
    tracing::info!("AO engine stopped cleanly.");

    Ok(())
}

/// Runs the continuous AO (Audio Output) processing loop for full System Audio:
/// - Captures system loopback audio from `dev_id`
/// - Passes audio through Win2Pod DSP (EQ, Enhancer, Warmth)
/// - Feeds real-time spectrum analyzer for Paint Pulse visualizer
/// - Renders enhanced audio to `ao_output_device` ONLY when it differs from the capture device
///   (Anti-Feedback Shield protects against acoustic / digital feedback loops)
pub fn run_ao_system_engine(
    dev_id: String,
    ao_output_device: Option<String>,
    stop: Arc<AtomicBool>,
    eq_params: Arc<EqParams>,
    enhancer_params: Arc<SoundEnhancerParams>,
    warmth_params: Arc<WarmthEnhancerParams>,
) -> anyhow::Result<()> {
    let _com = wasapi::initialize_mta();
    tracing::info!(?dev_id, ?ao_output_device, "Starting AO (System Loopback) engine");

    let is_system_mix = dev_id == "default" || dev_id.is_empty();
    let mut eq = Equalizer::new(EqConfig::ten_band(), eq_params, RATE);
    let mut enhancer = SoundEnhancer::new(enhancer_params, RATE);
    let mut warmth = WarmthEnhancer::new(warmth_params, RATE);

    let bytes_per_frame = CHANNELS as usize * 4; // 32-bit float stereo = 8 bytes per frame

    while !stop.load(Ordering::Relaxed) {
        let enumerator = match DeviceEnumerator::new() {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to create DeviceEnumerator: {e}");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let device = if is_system_mix {
            match enumerator.get_default_device(&Direction::Render) {
                Ok(d) => {
                    let is_airpods = d.get_friendlyname().map(|n| crate::controller::is_airpods_playback_device(&n)).unwrap_or(false);
                    if is_airpods {
                        if let Some(standin_id) = crate::controller::get_standin_device_id() {
                            tracing::warn!("AO System: Default device is AirPods [{:?}]; diverting capture to stand-in [{standin_id}]", d.get_id().ok());
                            let _ = crate::policy_config::set_default_render_device(&standin_id);
                            crate::controller::shield_endpoint(Some(&standin_id));
                            enumerator.get_device(&standin_id).unwrap_or(d)
                        } else {
                            d
                        }
                    } else {
                        d
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get default render device: {e}");
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        } else {
            match enumerator.get_device(&dev_id) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Target device [{dev_id}] not ready: {e}");
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        };

        let capture_id = device.get_id().unwrap_or_default();

        // Anti-Feedback Shield: Identify the physical render endpoint
        let out_id_str = ao_output_device.as_deref().unwrap_or("default");
        let render_dev = if out_id_str == "default" || out_id_str.is_empty() {
            enumerator.get_default_device(&Direction::Render).ok()
        } else {
            enumerator.get_device(out_id_str).ok()
        };
        let render_id = render_dev.as_ref().and_then(|d| d.get_id().ok()).unwrap_or_default();

        let is_feedback_risk = !capture_id.is_empty() && capture_id.eq_ignore_ascii_case(&render_id);

        let mut renderer: Option<LocalAudioRenderer> = if is_feedback_risk {
            tracing::info!("AO System: Capture [{capture_id}] matches Render [{render_id}] - Feedback Shield Active (suppressing local render to avoid feedback loop)");
            None
        } else {
            tracing::info!("AO System: Routing enhanced audio to [{render_id}] (Capture [{capture_id}] != Render [{render_id}])");
            match LocalAudioRenderer::new(ao_output_device.as_deref()) {
                Ok(r) => Some(r),
                Err(e) => {
                    tracing::warn!("AO System: Initial LocalAudioRenderer::new failed ({e}), will retry in background");
                    None
                }
            }
        };

        let mut last_render_retry = std::time::Instant::now();
        let mut render_stall_count = 0u32;

        let mut audio_client = match device.get_iaudioclient() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get IAudioClient for system AO: {e}");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let format = WaveFormat::new(32, 32, &SampleType::Float, RATE as usize, CHANNELS as usize, None);
        let (_default_period, min_period) = match audio_client.get_device_period() {
            Ok(p) => p,
            Err(_) => (100_000, 100_000),
        };
        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: min_period,
        };

        if let Err(e) = audio_client.initialize_client(&format, &Direction::Capture, &mode) {
            tracing::warn!("Failed to initialize audio client: {e}");
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        let h_event = match audio_client.set_get_eventhandle() {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Failed to set event handle: {e}");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let capture_client = match audio_client.get_audiocaptureclient() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get capture client: {e}");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let mut queue: VecDeque<u8> = VecDeque::new();
        if let Err(e) = audio_client.start_stream() {
            tracing::warn!("Failed to start system loopback stream: {e}");
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        tracing::info!("System AO loopback active! Driving visualizer & spectrum.");

        while !stop.load(Ordering::Relaxed) {
            if crate::controller::DEFAULT_DEVICE_CHANGED.swap(false, Ordering::SeqCst) {
                tracing::info!("Audio device change signaled, recycling system AO capture...");
                renderer = None;
                break;
            }

            // Zero-CPU Smart Sleep:
            // When capture and render devices are the same (Feedback Shield active, renderer is None),
            // and neither the flyout panel nor the visualizer window is open, there is zero audio output
            // and zero visible UI. Skip all DSP, FFT, and sample conversions.
            if is_feedback_risk && renderer.is_none() && !crate::tray::is_any_window_visible() {
                queue.clear();
                crate::spectrum::reset_spectrum_levels();
                crate::controller::set_audio_peak_level(0.0);
                std::thread::sleep(Duration::from_millis(150));
                continue;
            }

            // Periodic retry to establish / recover LocalAudioRenderer if it failed or was dropped
            if !is_feedback_risk && renderer.is_none() && last_render_retry.elapsed() >= Duration::from_millis(350) {
                last_render_retry = std::time::Instant::now();
                match LocalAudioRenderer::new(ao_output_device.as_deref()) {
                    Ok(r) => {
                        tracing::info!("AO System: LocalAudioRenderer successfully initialized!");
                        renderer = Some(r);
                        render_stall_count = 0;
                    }
                    Err(e) => {
                        tracing::debug!("AO System: Retrying LocalAudioRenderer initialization: {e}");
                    }
                }
            }

            match capture_client.read_from_device_to_deque(&mut queue) {
                Ok(_) => {
                    let n_frames = queue.len() / bytes_per_frame;
                    if n_frames > 0 {
                        let n_bytes = n_frames * bytes_per_frame;
                        let mut samples: Vec<i16> = Vec::with_capacity(n_frames * CHANNELS as usize);

                        let slice = &queue.make_contiguous()[..n_bytes];
                        for b in slice.chunks_exact(4) {
                            let f = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                            let s = (f.clamp(-1.0, 1.0) * 32767.0) as i16;
                            samples.push(s);
                        }
                        queue.drain(..n_bytes);

                        enhancer.process(&mut samples);
                        eq.process(&mut samples);
                        warmth.process(&mut samples);

                        // Balance and Mono Mix
                        let is_mono = crate::controller::is_audio_mono();
                        let bal = crate::controller::get_audio_balance();
                        if is_mono || bal != 0.0 {
                            for chunk in samples.chunks_exact_mut(2) {
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

                        crate::spectrum::process_post_dsp_pcm(&samples);

                        if let Some(ref r) = renderer {
                            let mut offset = 0;
                            let mut retries = 0;
                            let mut write_err = false;
                            while offset < samples.len() && retries < 3 {
                                match r.write_frames(&samples[offset..]) {
                                    Ok(written_frames) if written_frames > 0 => {
                                        offset += written_frames * CHANNELS as usize;
                                        retries = 0;
                                        render_stall_count = 0;
                                    }
                                    Ok(_) => {
                                        retries += 1;
                                        std::thread::sleep(Duration::from_millis(2));
                                    }
                                    Err(e) => {
                                        tracing::warn!("AO system render write error: {e}");
                                        write_err = true;
                                        break;
                                    }
                                }
                            }
                            if write_err {
                                tracing::warn!("AO system: render write error, resetting renderer for re-initialization");
                                renderer = None;
                                last_render_retry = std::time::Instant::now();
                            } else if offset == 0 && !samples.is_empty() {
                                render_stall_count += 1;
                                if render_stall_count > 15 {
                                    tracing::warn!("AO system: render buffer stalled, recreating LocalAudioRenderer");
                                    renderer = None;
                                    render_stall_count = 0;
                                    last_render_retry = std::time::Instant::now();
                                }
                            }
                        } else {
                            // Feedback Shield Active or no render target:
                            // We MUST pace the loop to match real-time audio playback!
                            // n_frames at RATE Hz represents (n_frames * 1000 / RATE) ms.
                            // Sleeping or waiting on event here prevents an uncontrolled 100% CPU runaway loop!
                            let frame_ms = ((n_frames as u64 * 1000) / RATE as u64).max(5);
                            let _ = h_event.wait_for_event(frame_ms as u32);
                        }
                    } else {
                        let _ = h_event.wait_for_event(40);
                    }
                }
                Err(e) => {
                    crate::spectrum::reset_spectrum_levels();
                    crate::controller::set_audio_peak_level(0.0);
                    tracing::warn!("System AO capture error: {e:#}, retrying...");
                    std::thread::sleep(Duration::from_millis(300));
                    break;
                }
            }
        }

        let _ = audio_client.stop_stream();
    }

    crate::spectrum::reset_spectrum_levels();
    crate::controller::set_audio_peak_level(0.0);
    tracing::info!("System AO engine stopped cleanly.");

    Ok(())
}
