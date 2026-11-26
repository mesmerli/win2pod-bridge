//! Unified WASAPI Audio Manager for win2pod-engine.
//!
//! Centralizes Windows Audio Session API (WASAPI) and Core Audio COM operations:
//! - Render endpoint enumeration and default device tracking
//! - Hardware speaker mute / unmute via IAudioEndpointVolume
//! - Loopback audio capture, spectrum FFT analysis, and 30s Eco Mode

use std::sync::Arc;
use anyhow::{Context, Result};
use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use wasapi::{initialize_mta, DeviceEnumerator, Direction};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use win2pod_ipc::protocol::AudioEndpointIpc;

const FFT_SIZE: usize = 512;
pub const RAW_SPECTRUM_SIZE: usize = 256;

const WIN2POD_EVENT_CONTEXT: windows::core::GUID = windows::core::GUID::from_u128(0x5732506F_6445_7665_6E74_436F6E746578);

/// Enumerate all active render endpoints on the system.
pub fn enumerate_render_endpoints() -> Result<Vec<AudioEndpointIpc>> {
    let _com = initialize_mta();
    let enumerator = DeviceEnumerator::new().context("Failed to create DeviceEnumerator")?;

    let default_id = enumerator
        .get_default_device(&Direction::Render)
        .ok()
        .and_then(|d| d.get_id().ok())
        .unwrap_or_default();

    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .context("Failed to get render device collection")?;

    let count = collection.get_nbr_devices().unwrap_or(0);
    let mut endpoints = Vec::new();

    for i in 0..count {
        if let Ok(dev) = collection.get_device_at_index(i) {
            if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                if let (Ok(id), Ok(name)) = (dev.get_id(), dev.get_friendlyname()) {
                    let is_default = id.eq_ignore_ascii_case(&default_id);
                    endpoints.push(AudioEndpointIpc {
                        id,
                        name,
                        is_default,
                    });
                }
            }
        }
    }

    Ok(endpoints)
}

/// Set hardware speaker mute via IAudioEndpointVolume.
pub fn set_speaker_mute(mute: bool, device_id: Option<&str>) -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .context("CoCreateInstance MMDeviceEnumerator failed")?;

        if let Some(id) = device_id {
            if id != "default" && !id.is_empty() {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                let device = enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))
                    .context("GetDevice failed")?;
                let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)
                    .context("Activate IAudioEndpointVolume failed")?;
                let _ = ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT);
                return Ok(());
            }
        }

        // Default console & multimedia endpoints
        if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                let _ = ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT);
            }
        }
        if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
            if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                let _ = ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT);
            }
        }
    }
    Ok(())
}

/// Real-time Spectrum Analyzer & Audio Activity Tracker for Engine.
pub struct SpectrumEngine {
    fft_forward: Arc<dyn RealToComplex<f32>>,
    fft_in: Vec<f32>,
    fft_out: Vec<Complex<f32>>,
    window: Vec<f32>,
    buffer: Vec<f32>,
}

impl SpectrumEngine {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(FFT_SIZE);
        let fft_in = fft_forward.make_input_vec();
        let fft_out = fft_forward.make_output_vec();

        // Hann window
        let mut window = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let v = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            window.push(v);
        }

        Self {
            fft_forward,
            fft_in,
            fft_out,
            window,
            buffer: Vec::with_capacity(FFT_SIZE * 2),
        }
    }

    pub fn process_samples(&mut self, mono_samples: &[f32]) -> Option<[u8; RAW_SPECTRUM_SIZE]> {
        self.buffer.extend_from_slice(mono_samples);
        if self.buffer.len() < FFT_SIZE {
            return None;
        }

        // Drain FFT_SIZE samples
        let chunk: Vec<f32> = self.buffer.drain(..FFT_SIZE).collect();
        for i in 0..FFT_SIZE {
            self.fft_in[i] = chunk[i] * self.window[i];
        }

        if self.fft_forward.process(&mut self.fft_in, &mut self.fft_out).is_err() {
            return None;
        }

        let mut raw = [0u8; RAW_SPECTRUM_SIZE];
        for bin in 0..RAW_SPECTRUM_SIZE {
            if bin < self.fft_out.len() {
                let c = self.fft_out[bin];
                let mag = (c.re * c.re + c.im * c.im).sqrt();
                let norm = mag / (FFT_SIZE as f32 * 0.25);
                let db = 20.0 * (norm + 1e-5).log10();
                let level = ((db + 55.0) / 55.0).clamp(0.0, 1.0);
                raw[bin] = (level * 255.0) as u8;
            }
        }

        Some(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_engine_fft_processing() {
        let mut engine = SpectrumEngine::new();

        // 1. Partial input should return None until 512 samples are accumulated
        let half = vec![0.0f32; 256];
        assert!(engine.process_samples(&half).is_none());

        // 2. Complete 512 with silence
        let res = engine.process_samples(&half);
        assert!(res.is_some());
        let raw = res.unwrap();
        assert_eq!(raw.len(), RAW_SPECTRUM_SIZE);
        // Silence should be 0 across all bins
        assert!(raw.iter().all(|&v| v == 0));

        // 3. Generate 1 kHz sine wave at 44.1 kHz
        let mut sine = Vec::with_capacity(512);
        for i in 0..512 {
            let t = i as f32 / 44100.0;
            sine.push((2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.8);
        }

        let res_sine = engine.process_samples(&sine).expect("Should produce spectrum for 512 samples");
        // Bin for 1 kHz is around 1000 / (44100 / 512) = 11.6 -> bins 11 or 12
        let peak_bin = res_sine.iter().enumerate().max_by_key(|&(_, &val)| val).map(|(idx, _)| idx).unwrap();
        assert!(peak_bin >= 10 && peak_bin <= 13, "Peak bin was expected around 11-12, got {peak_bin}");
        assert!(res_sine[peak_bin] > 180, "Expected strong magnitude at peak bin, got {}", res_sine[peak_bin]);
    }
}
