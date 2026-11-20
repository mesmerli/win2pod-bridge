//! Audio sink interfaces and implementations for AirPlay receiver.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use airplay_audio::{LiveFrameSender, LivePcmFrame, SoftwareAudioMixer};
#[cfg(windows)]
use airplay_audio::{EqConfig, Equalizer, SoundEnhancer, WarmthEnhancer};

#[cfg(windows)]
use windows::Win32::Media::Audio::*;
#[cfg(windows)]
use windows::Win32::System::Com::*;

/// Trait for receiving audio samples and stream events from the AirPlay receiver.
pub trait AudioSink: Send + Sync {
    /// Delivered when fresh decoded PCM samples are available.
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8);

    /// Delivered when sender adjusts volume (dB, from -144.0 to 0.0).
    fn on_volume_change(&self, volume_db: f32) {
        let _ = volume_db;
    }

    /// Delivered when audio streaming starts (RTSP RECORD).
    fn on_stream_start(&self) {}

    /// Delivered when audio streaming starts (RTSP RECORD), optionally with client device name.
    fn on_stream_start_with_client(&self, client_name: Option<&str>) {
        let _ = client_name;
        self.on_stream_start();
    }

    /// Delivered when sender device name is resolved or updated.
    fn on_client_name_change(&self, name: &str) {
        let _ = name;
    }

    /// Delivered when sender connects (RTSP ANNOUNCE/SETUP) with device info.
    fn on_client_connected(&self, client_name: &str, device_model: &str, client_ip: &str) {
        let _ = (client_name, device_model, client_ip);
    }

    /// Delivered when audio streaming is paused (RTSP PAUSE).
    fn on_stream_paused(&self) {}

    /// Delivered when sender completely disconnects (TCP connection closed).
    fn on_client_disconnected(&self) {}

    /// Delivered when track metadata (title, artist, album) is updated.
    fn on_metadata_change(&self, title: Option<&str>, artist: Option<&str>, album: Option<&str>) {
        let _ = (title, artist, album);
    }

    /// Delivered when album artwork is updated (JPEG/PNG bytes, or None when cleared).
    fn on_artwork_change(&self, artwork_bytes: Option<&[u8]>) {
        let _ = artwork_bytes;
    }

    /// Delivered when audio streaming stops (RTSP TEARDOWN or disconnect).
    fn on_stream_stop(&self) {}

    /// Delivered when stream flush is requested (RTSP FLUSH).
    fn on_flush(&self) {}

    /// Reported Audio-Latency in frames (samples per channel) for RTSP RECORD response.
    fn latency_frames(&self) -> u32 {
        66150
    }
}

/// Parameters for processing DSP effects on received audio before playback.
#[derive(Clone)]
pub struct ReceiverDspParams {
    pub apply_effects: Arc<AtomicBool>,
    pub eq_params: Arc<airplay_audio::EqParams>,
    pub sound_enhancer_params: Arc<airplay_audio::SoundEnhancerParams>,
    pub warmth_enhancer_params: Arc<airplay_audio::WarmthEnhancerParams>,
}

#[cfg(windows)]
enum SpeakerMsg {
    Samples(Vec<i16>),
    Start,
    Stop,
    Flush,
}

/// Sink that renders audio directly to local PC speakers via Windows WASAPI.
#[cfg(windows)]
pub struct LocalSpeakerSink {
    sender: std::sync::mpsc::SyncSender<SpeakerMsg>,
    stop_signal: Arc<AtomicBool>,
}

#[cfg(windows)]
impl LocalSpeakerSink {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_device_and_dsp(None, None)
    }

    pub fn new_with_device(device_id: Option<String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_device_and_dsp(device_id, None)
    }

    pub fn new_with_device_and_dsp(
        device_id: Option<String>,
        dsp_params: Option<ReceiverDspParams>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<SpeakerMsg>(512);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = Arc::clone(&stop_signal);

        std::thread::Builder::new()
            .name("airplay-speaker-sink".into())
            .spawn(move || {
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                }

                let mut equalizer = dsp_params.as_ref().map(|d| {
                    Equalizer::new(EqConfig::ten_band(), Arc::clone(&d.eq_params), 44100)
                });
                let mut sound_enhancer = dsp_params.as_ref().map(|d| {
                    SoundEnhancer::new(Arc::clone(&d.sound_enhancer_params), 44100)
                });
                let mut warmth_enhancer = dsp_params.as_ref().map(|d| {
                    WarmthEnhancer::new(Arc::clone(&d.warmth_enhancer_params), 44100)
                });

                let dev_enumerator: Option<IMMDeviceEnumerator> = unsafe {
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()
                };
                let mut renderer: Option<WasapiRenderer> = None;
                let mut pending = std::collections::VecDeque::with_capacity(16384);
                let mut last_default_check = std::time::Instant::now();
                let is_tracking_default = device_id.as_deref().map_or(true, |id| id.is_empty() || id == "default");

                while !stop_signal_clone.load(Ordering::Relaxed) {
                    let msg = if pending.is_empty() {
                        match rx.recv_timeout(Duration::from_millis(10)) {
                            Ok(m) => Some(m),
                            Err(_) => None,
                        }
                    } else {
                        rx.try_recv().ok()
                    };

                    if let Some(msg) = msg {
                        match msg {
                            SpeakerMsg::Start => {
                                renderer = None;
                                pending.clear();
                                if let Some(ref mut eq) = equalizer { eq.reset(); }
                                if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                match WasapiRenderer::new(device_id.as_deref()) {
                                    Ok(r) => {
                                        tracing::info!("LocalSpeakerSink: Windows WASAPI renderer initialized on stream start (device: '{}')", r.device_id);
                                        renderer = Some(r);
                                    }
                                    Err(e) => {
                                        tracing::error!("LocalSpeakerSink: Failed to initialize WASAPI on start: {}", e);
                                    }
                                }
                            }
                            SpeakerMsg::Stop => {
                                renderer = None;
                                pending.clear();
                                if let Some(ref mut eq) = equalizer { eq.reset(); }
                                if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                tracing::info!("LocalSpeakerSink: Windows WASAPI renderer stopped");
                            }
                            SpeakerMsg::Flush => {
                                pending.clear();
                                if let Some(ref mut eq) = equalizer { eq.reset(); }
                                if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                tracing::debug!("LocalSpeakerSink: stream flush");
                            }
                            SpeakerMsg::Samples(mut samples) => {
                                if let Some(ref dsp) = dsp_params {
                                    if dsp.apply_effects.load(Ordering::Relaxed) {
                                        if let Some(ref mut eq) = equalizer {
                                            eq.process(&mut samples);
                                        }
                                        if let Some(ref mut se) = sound_enhancer {
                                            se.process(&mut samples);
                                        }
                                        if let Some(ref mut we) = warmth_enhancer {
                                            we.process(&mut samples);
                                        }
                                    }
                                }
                                pending.extend(samples);
                                while let Ok(m) = rx.try_recv() {
                                    match m {
                                        SpeakerMsg::Samples(mut s) => {
                                            if let Some(ref dsp) = dsp_params {
                                                if dsp.apply_effects.load(Ordering::Relaxed) {
                                                    if let Some(ref mut eq) = equalizer {
                                                        eq.process(&mut s);
                                                    }
                                                    if let Some(ref mut se) = sound_enhancer {
                                                        se.process(&mut s);
                                                    }
                                                    if let Some(ref mut we) = warmth_enhancer {
                                                        we.process(&mut s);
                                                    }
                                                }
                                            }
                                            pending.extend(s);
                                        }
                                        SpeakerMsg::Flush => {
                                            pending.clear();
                                            if let Some(ref mut eq) = equalizer { eq.reset(); }
                                            if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                            if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                        }
                                        SpeakerMsg::Stop => {
                                            renderer = None;
                                            pending.clear();
                                            if let Some(ref mut eq) = equalizer { eq.reset(); }
                                            if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                            if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                            break;
                                        }
                                        SpeakerMsg::Start => {
                                            renderer = None;
                                            pending.clear();
                                            if let Some(ref mut eq) = equalizer { eq.reset(); }
                                            if let Some(ref mut se) = sound_enhancer { se.reset(); }
                                            if let Some(ref mut we) = warmth_enhancer { we.reset(); }
                                            if let Ok(r) = WasapiRenderer::new(device_id.as_deref()) {
                                                renderer = Some(r);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if pending.is_empty() {
                        continue;
                    }

                    // If target device is default, dynamically track default render endpoint changes
                    // (e.g. PC wakes from sleep, HDMI monitor connects/disconnects, headphones plugged/unplugged)
                    if is_tracking_default && last_default_check.elapsed() >= Duration::from_millis(300) {
                        last_default_check = std::time::Instant::now();
                        if let Some(ref current_r) = renderer {
                            if let Some(ref enum_client) = dev_enumerator {
                                if let Ok(default_endpoint) = unsafe { enum_client.GetDefaultAudioEndpoint(eRender, eMultimedia) } {
                                    if let Ok(pwstr) = unsafe { default_endpoint.GetId() } {
                                        if let Ok(curr_default_id) = unsafe { pwstr.to_string() } {
                                            if !curr_default_id.is_empty() && curr_default_id != current_r.device_id {
                                                tracing::info!(
                                                    "LocalSpeakerSink: Default render endpoint changed from '{}' to '{}', re-routing...",
                                                    current_r.device_id, curr_default_id
                                                );
                                                renderer = None;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if renderer.is_none() {
                        match WasapiRenderer::new(device_id.as_deref()) {
                            Ok(r) => {
                                tracing::info!("LocalSpeakerSink: Windows WASAPI renderer initialized on-demand (device: '{}')", r.device_id);
                                renderer = Some(r);
                            }
                            Err(e) => {
                                tracing::error!("LocalSpeakerSink: On-demand WASAPI init failed: {}", e);
                                pending.clear();
                                std::thread::sleep(Duration::from_millis(50));
                                continue;
                            }
                        }
                    }

                    const MAX_PENDING_SAMPLES: usize = 44100 * 2;
                    if pending.len() > MAX_PENDING_SAMPLES {
                        let excess = pending.len() - 44100;
                        pending.drain(..excess);
                    }

                    if let Some(ref r) = renderer {
                        let chunk_size = pending.len().min(4096);
                        let mut chunk = Vec::with_capacity(chunk_size);
                        for _ in 0..chunk_size {
                            if let Some(s) = pending.pop_front() {
                                chunk.push(s);
                            }
                        }

                        let mut offset = 0;
                        while offset < chunk.len() && !stop_signal_clone.load(Ordering::Relaxed) {
                            match r.write_frames(&chunk[offset..]) {
                                Ok(frames_written) => {
                                    if frames_written == 0 {
                                        std::thread::sleep(Duration::from_millis(2));
                                    } else {
                                        offset += frames_written * 2;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("LocalSpeakerSink: WASAPI write error: {}", e);
                                    renderer = None;
                                    break;
                                }
                            }
                        }
                    }
                }

                unsafe {
                    CoUninitialize();
                }
            })?;

        Ok(Self {
            sender: tx,
            stop_signal,
        })
    }
}

#[cfg(windows)]
impl Drop for LocalSpeakerSink {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

#[cfg(windows)]
impl AudioSink for LocalSpeakerSink {
    fn on_audio_samples(&self, samples: &[i16], _sample_rate: u32, _channels: u8) {
        let _ = self.sender.try_send(SpeakerMsg::Samples(samples.to_vec()));
    }

    fn on_stream_start(&self) {
        let _ = self.sender.try_send(SpeakerMsg::Start);
    }

    fn on_stream_stop(&self) {
        let _ = self.sender.try_send(SpeakerMsg::Stop);
    }

    fn on_flush(&self) {
        let _ = self.sender.try_send(SpeakerMsg::Flush);
    }
}

#[cfg(windows)]
struct WasapiRenderer {
    client: IAudioClient,
    render_client: IAudioRenderClient,
    buffer_frame_count: u32,
    channels: u16,
    pub device_id: String,
}

#[cfg(windows)]
impl WasapiRenderer {
    fn new(device_id: Option<&str>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("CoCreateInstance: {}", e))?;
            let device = match device_id {
                Some(id) if !id.is_empty() && id != "default" => {
                    let hstring = windows::core::HSTRING::from(id);
                    enumerator.GetDevice(&hstring)
                        .map_err(|e| format!("GetDevice({}): {}", id, e))?
                }
                _ => enumerator
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .map_err(|e| format!("GetDefaultAudioEndpoint: {}", e))?,
            };
            let dev_id_str = match device.GetId() {
                Ok(pwstr) => pwstr.to_string().unwrap_or_default(),
                Err(_) => String::new(),
            };
            let client: IAudioClient = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate: {}", e))?;

            let sample_rate = 44100u32;
            let channels = 2u16;
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

            let stream_flags = 0x80000000 | 0x08000000; // AUTOCONVERTPCM | SRC_DEFAULT_QUALITY
            let hns_buffer_duration: i64 = 1_000_000; // 100ms buffer

            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    stream_flags,
                    hns_buffer_duration,
                    0,
                    &wfx as *const _ as *const _,
                    None,
                )
                .map_err(|e| format!("Initialize: {}", e))?;

            let buffer_frame_count = client
                .GetBufferSize()
                .map_err(|e| format!("GetBufferSize: {}", e))?;
            let render_client: IAudioRenderClient = client
                .GetService()
                .map_err(|e| format!("GetService: {}", e))?;

            client.Start().map_err(|e| format!("Start: {}", e))?;

            Ok(Self {
                client,
                render_client,
                buffer_frame_count,
                channels,
                device_id: dev_id_str,
            })
        }
    }

    fn write_frames(&self, pcm_samples: &[i16]) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let frames_to_write = (pcm_samples.len() / self.channels as usize) as u32;
        if frames_to_write == 0 {
            return Ok(0);
        }

        unsafe {
            let padding = self.client.GetCurrentPadding()
                .map_err(|e| format!("GetCurrentPadding: {}", e))?;
            let available = self.buffer_frame_count.saturating_sub(padding);
            let num_frames = frames_to_write.min(available);

            if num_frames == 0 {
                return Ok(0);
            }

            let p_data = self.render_client.GetBuffer(num_frames)
                .map_err(|e| format!("GetBuffer: {}", e))?;
            let bytes_to_copy = num_frames as usize * self.channels as usize * 2;
            let src_bytes = std::slice::from_raw_parts(
                pcm_samples.as_ptr() as *const u8,
                bytes_to_copy,
            );
            std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), p_data, bytes_to_copy);

            self.render_client.ReleaseBuffer(num_frames, 0)
                .map_err(|e| format!("ReleaseBuffer: {}", e))?;
            Ok(num_frames as usize)
        }
    }
}

#[cfg(windows)]
impl Drop for WasapiRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.Stop();
        }
    }
}

/// Fallback / Non-windows LocalSpeakerSink
#[cfg(not(windows))]
pub struct LocalSpeakerSink;

#[cfg(not(windows))]
impl LocalSpeakerSink {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self)
    }
}

#[cfg(not(windows))]
impl AudioSink for LocalSpeakerSink {
    fn on_audio_samples(&self, _samples: &[i16], _sample_rate: u32, _channels: u8) {}
}

/// Sink that pushes audio into a `LiveFrameSender` for HomePod / AirPlay 2 re-casting.
pub struct LiveFrameSink {
    sender: LiveFrameSender,
}

impl LiveFrameSink {
    pub fn new(sender: LiveFrameSender) -> Self {
        Self { sender }
    }
}

impl AudioSink for LiveFrameSink {
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8) {
        let frame = LivePcmFrame {
            samples: samples.to_vec(),
            channels,
            sample_rate,
        };
        self.sender.try_send(frame);
    }
}

/// Tee sink that splits audio to multiple child sinks (e.g. Speakers + HomePod).
pub struct TeeSink {
    sinks: Vec<Arc<dyn AudioSink>>,
}

impl TeeSink {
    pub fn new(sinks: Vec<Arc<dyn AudioSink>>) -> Self {
        Self { sinks }
    }
}

impl AudioSink for TeeSink {
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8) {
        for sink in &self.sinks {
            sink.on_audio_samples(samples, sample_rate, channels);
        }
    }

    fn on_volume_change(&self, volume_db: f32) {
        for sink in &self.sinks {
            sink.on_volume_change(volume_db);
        }
    }

    fn on_stream_start(&self) {
        for sink in &self.sinks {
            sink.on_stream_start();
        }
    }

    fn on_stream_start_with_client(&self, client_name: Option<&str>) {
        for sink in &self.sinks {
            sink.on_stream_start_with_client(client_name);
        }
    }

    fn on_client_name_change(&self, name: &str) {
        for sink in &self.sinks {
            sink.on_client_name_change(name);
        }
    }

    fn on_client_connected(&self, client_name: &str, device_model: &str, client_ip: &str) {
        for sink in &self.sinks {
            sink.on_client_connected(client_name, device_model, client_ip);
        }
    }

    fn on_stream_paused(&self) {
        for sink in &self.sinks {
            sink.on_stream_paused();
        }
    }

    fn on_client_disconnected(&self) {
        for sink in &self.sinks {
            sink.on_client_disconnected();
        }
    }

    fn on_metadata_change(&self, title: Option<&str>, artist: Option<&str>, album: Option<&str>) {
        for sink in &self.sinks {
            sink.on_metadata_change(title, artist, album);
        }
    }

    fn on_artwork_change(&self, artwork_bytes: Option<&[u8]>) {
        for sink in &self.sinks {
            sink.on_artwork_change(artwork_bytes);
        }
    }

    fn on_stream_stop(&self) {
        for sink in &self.sinks {
            sink.on_stream_stop();
        }
    }

    fn on_flush(&self) {
        for sink in &self.sinks {
            sink.on_flush();
        }
    }
}

/// Simple closure-based callback sink.
pub struct CallbackSink<F>
where
    F: Fn(&[i16], u32, u8) + Send + Sync,
{
    callback: F,
}

impl<F> CallbackSink<F>
where
    F: Fn(&[i16], u32, u8) + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> AudioSink for CallbackSink<F>
where
    F: Fn(&[i16], u32, u8) + Send + Sync,
{
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8) {
        (self.callback)(samples, sample_rate, channels);
    }
}

/// Sink that pushes audio samples into a `SoftwareAudioMixer` for real-time mixing with PC audio.
pub struct MixerSink {
    mixer: Arc<SoftwareAudioMixer>,
}

impl MixerSink {
    pub fn new(mixer: Arc<SoftwareAudioMixer>) -> Self {
        Self { mixer }
    }
}

impl AudioSink for MixerSink {
    fn on_audio_samples(&self, samples: &[i16], _sample_rate: u32, _channels: u8) {
        self.mixer.push_secondary(samples);
    }

    fn on_flush(&self) {
        self.mixer.clear_secondary();
    }

    fn on_stream_stop(&self) {
        self.mixer.clear_secondary();
    }
}

/// Sink that allows hot-swapping the active destination sink at runtime without disrupting RTSP sessions,
/// while providing FIFO-based A/V sync compensation delay.
#[derive(Clone)]
pub struct SwitchableSink {
    inner: Arc<std::sync::RwLock<Arc<dyn AudioSink>>>,
    target_delay_samples: Arc<AtomicUsize>,
    buffer: Arc<Mutex<VecDeque<i16>>>,
}

impl SwitchableSink {
    pub fn new(initial: Arc<dyn AudioSink>) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(initial)),
            target_delay_samples: Arc::new(AtomicUsize::new(132300)), // Default 1500ms (66150 frames * 2) - perfect sync for YouTube
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(264600))),
        }
    }

    pub fn set_target(&self, sink: Arc<dyn AudioSink>) {
        if let Ok(mut lock) = self.inner.write() {
            *lock = sink;
        }
    }

    pub fn set_delay_samples(&self, samples: usize) {
        self.target_delay_samples.store(samples, Ordering::Relaxed);
        if samples == 0 {
            if let Ok(mut buf) = self.buffer.lock() {
                if !buf.is_empty() {
                    let drained: Vec<i16> = buf.drain(..).collect();
                    if let Ok(lock) = self.inner.read() {
                        lock.on_audio_samples(&drained, 44100, 2);
                    }
                }
            }
        }
    }
}

impl AudioSink for SwitchableSink {
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8) {
        let target_delay = self.target_delay_samples.load(Ordering::Relaxed);
        if target_delay == 0 {
            if let Ok(lock) = self.inner.read() {
                lock.on_audio_samples(samples, sample_rate, channels);
            }
            return;
        }

        let to_emit = if let Ok(mut buf) = self.buffer.lock() {
            buf.extend(samples.iter().copied());
            // Guard against unbounded queue in case of network bursts (cap at 3 seconds)
            const MAX_BUFFER: usize = 44100 * 2 * 3;
            if buf.len() > MAX_BUFFER {
                let drop_len = buf.len() - target_delay;
                buf.drain(..drop_len);
            }
            if buf.len() > target_delay {
                let excess = buf.len() - target_delay;
                let chunk: Vec<i16> = buf.drain(..excess).collect();
                Some(chunk)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(chunk) = to_emit {
            if let Ok(lock) = self.inner.read() {
                lock.on_audio_samples(&chunk, sample_rate, channels);
            }
        }
    }

    fn on_volume_change(&self, volume_db: f32) {
        if let Ok(lock) = self.inner.read() {
            lock.on_volume_change(volume_db);
        }
    }

    fn on_stream_start(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        if let Ok(lock) = self.inner.read() {
            lock.on_stream_start();
        }
    }

    fn on_stream_start_with_client(&self, client_name: Option<&str>) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        if let Ok(lock) = self.inner.read() {
            lock.on_stream_start_with_client(client_name);
        }
    }

    fn on_client_name_change(&self, name: &str) {
        if let Ok(lock) = self.inner.read() {
            lock.on_client_name_change(name);
        }
    }

    fn on_client_connected(&self, client_name: &str, device_model: &str, client_ip: &str) {
        if let Ok(lock) = self.inner.read() {
            lock.on_client_connected(client_name, device_model, client_ip);
        }
    }

    fn on_stream_paused(&self) {
        if let Ok(lock) = self.inner.read() {
            lock.on_stream_paused();
        }
    }

    fn on_client_disconnected(&self) {
        if let Ok(lock) = self.inner.read() {
            lock.on_client_disconnected();
        }
    }

    fn on_metadata_change(&self, title: Option<&str>, artist: Option<&str>, album: Option<&str>) {
        if let Ok(lock) = self.inner.read() {
            lock.on_metadata_change(title, artist, album);
        }
    }

    fn on_artwork_change(&self, artwork_bytes: Option<&[u8]>) {
        if let Ok(lock) = self.inner.read() {
            lock.on_artwork_change(artwork_bytes);
        }
    }

    fn on_stream_stop(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        if let Ok(lock) = self.inner.read() {
            lock.on_stream_stop();
        }
    }

    fn on_flush(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        if let Ok(lock) = self.inner.read() {
            lock.on_flush();
        }
    }

    fn latency_frames(&self) -> u32 {
        let stereo_samples = self.target_delay_samples.load(Ordering::Relaxed);
        if stereo_samples == 0 {
            441 // ~10ms
        } else {
            (stereo_samples / 2) as u32
        }
    }
}
