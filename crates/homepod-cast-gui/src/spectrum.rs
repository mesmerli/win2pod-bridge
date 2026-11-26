//! Real-time Audio Spectrum Analyzer using 512-point Real FFT.
//! Computes 16 logarithmically spaced frequency bands for smooth visualization.

use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub const SPECTRUM_BANDS: usize = 16;
pub const RAW_SPECTRUM_SIZE: usize = 256;
const FFT_SIZE: usize = 512;

/// Global atomic array storing normalized spectrum amplitude (0.0 ..= 1.0) scaled by 10000.
pub static SPECTRUM_LEVELS: [AtomicU32; SPECTRUM_BANDS] = [
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
];

static RAW_SPECTRUM: Mutex<[u8; RAW_SPECTRUM_SIZE]> = Mutex::new([0u8; RAW_SPECTRUM_SIZE]);

static ANALYZER: Mutex<Option<SpectrumAnalyzer>> = Mutex::new(None);
static WAS_AUDIO_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static IS_SPECTRUM_NEEDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAST_AUDIO_ACTIVE_TS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Get the monotonic timestamp (ms) of the most recent non-silent audio activity.
pub fn get_last_audio_active_ts() -> u64 {
    LAST_AUDIO_ACTIVE_TS.load(Ordering::Relaxed)
}

/// Explicitly record audio activity timestamp and trigger zero-latency wake if needed.
pub fn record_audio_activity() {
    let now = crate::controller::monotonic_now_ms().max(1);
    LAST_AUDIO_ACTIVE_TS.store(now, Ordering::Relaxed);
    let prev = WAS_AUDIO_ACTIVE.swap(true, Ordering::Relaxed);
    if !prev {
        crate::gui::request_ui_repaint();
        crate::visualizer::wake_visualizer_from_eco();
    }
}

/// Checks whether non-silent audio was active within the last 30 seconds.
pub fn is_audio_active_30s() -> bool {
    let last = LAST_AUDIO_ACTIVE_TS.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    let now = crate::controller::monotonic_now_ms();
    now.saturating_sub(last) <= 30_000
}

/// Fast path query for the audio DSP thread:
/// Checks if any spectrum-consuming window is currently active and visible on screen.
/// Uses a single relaxed atomic read (< 1ns) with zero syscall / Win32 API overhead.
#[inline]
pub fn is_spectrum_needed() -> bool {
    IS_SPECTRUM_NEEDED.load(Ordering::Relaxed)
}

/// Evaluates whether any window requiring audio spectrum is currently visible and active,
/// updating the atomic cache flag. Returns the new state.
pub fn update_spectrum_needed_state() -> bool {
    let needed = crate::snapping::ALL_WINDOW_KINDS
        .iter()
        .any(|&kind| kind.consumes_spectrum() && crate::snapping::is_window_visible_and_active(kind));

    IS_SPECTRUM_NEEDED.store(needed, Ordering::SeqCst);
    needed
}

/// Set manual override for testing
pub fn set_spectrum_needed_override(needed: bool) {
    IS_SPECTRUM_NEEDED.store(needed, Ordering::SeqCst);
}

/// Process post-DSP audio samples (interleaved stereo i16) to update real-time spectrum and peak level.
pub fn process_post_dsp_pcm(samples: &[i16]) {
    if samples.is_empty() {
        return;
    }
    let mut mono = Vec::with_capacity(samples.len() / 2);
    let mut max_abs = 0.0f32;
    if samples.len() % 2 == 0 {
        for chunk in samples.chunks_exact(2) {
            let l = chunk[0] as f32 / 32768.0;
            let r = chunk[1] as f32 / 32768.0;
            let m = (l + r) * 0.5;
            let abs = m.abs();
            if abs > max_abs {
                max_abs = abs;
            }
            mono.push(m);
        }
    } else {
        for &s in samples {
            let m = s as f32 / 32768.0;
            let abs = m.abs();
            if abs > max_abs {
                max_abs = abs;
            }
            mono.push(m);
        }
    }

    crate::controller::set_audio_peak_level(max_abs);

    let is_active = max_abs > 0.001;
    let prev = WAS_AUDIO_ACTIVE.swap(is_active, Ordering::Relaxed);
    if is_active {
        LAST_AUDIO_ACTIVE_TS.store(crate::controller::monotonic_now_ms().max(1), Ordering::Relaxed);
        if !prev {
            // Audio transitioned from silence to active: wake UI and visualizer immediately for zero-latency animation
            crate::gui::request_ui_repaint();
            crate::visualizer::wake_visualizer_from_eco();
        }
    }

    if max_abs < 0.0001 {
        WAS_AUDIO_ACTIVE.store(false, Ordering::Relaxed);
        reset_spectrum_levels();
        return;
    }

    if !is_spectrum_needed() {
        return;
    }

    if let Ok(mut lock) = ANALYZER.lock() {
        if lock.is_none() {
            *lock = Some(SpectrumAnalyzer::new(44100));
        }
        if let Some(ref mut analyzer) = *lock {
            analyzer.feed_samples(&mono);
        }
    }
}

/// Get the current spectrum energy levels for all 16 frequency bands (0.0 ..= 1.0).
pub fn get_spectrum_levels() -> [f32; SPECTRUM_BANDS] {
    let mut out = [0.0; SPECTRUM_BANDS];
    for (i, val) in SPECTRUM_LEVELS.iter().enumerate() {
        out[i] = val.load(Ordering::Relaxed) as f32 / 10000.0;
    }
    out
}

/// Get the raw 256-bin FFT spectrum magnitudes (0 ..= 255) for high-resolution visualizers.
pub fn get_raw_spectrum() -> [u8; RAW_SPECTRUM_SIZE] {
    if let Ok(guard) = RAW_SPECTRUM.lock() {
        *guard
    } else {
        [0u8; RAW_SPECTRUM_SIZE]
    }
}

const BAND_BIN_RANGES: [(usize, usize); SPECTRUM_BANDS] = [
    (0, 1),
    (1, 2),
    (1, 2),
    (1, 3),
    (2, 5),
    (4, 7),
    (6, 12),
    (11, 18),
    (17, 28),
    (27, 45),
    (44, 70),
    (69, 99),
    (98, 134),
    (133, 175),
    (174, 215),
    (214, 255),
];

/// Update raw 256-bin FFT spectrum directly from win2pod-engine IPC push.
pub fn update_raw_spectrum_from_engine(raw: &[u8]) {
    if raw.len() == RAW_SPECTRUM_SIZE {
        let mut arr = [0u8; RAW_SPECTRUM_SIZE];
        arr.copy_from_slice(raw);
        if let Ok(mut guard) = RAW_SPECTRUM.lock() {
            *guard = arr;
        }

        let mut has_audio = false;

        // Downsample the 256 bins into the 16 bands for GUI meters
        for (band_idx, &(start_bin, end_bin)) in BAND_BIN_RANGES.iter().enumerate() {
            let mut max_val = 0u8;
            for bin in start_bin..=end_bin.min(RAW_SPECTRUM_SIZE - 1) {
                if raw[bin] > max_val {
                    max_val = raw[bin];
                }
            }
            if max_val > 0 {
                has_audio = true;
            }
            let norm_val = max_val as f32 / 255.0;
            // Perceptual tilt consistent with SpectrumAnalyzer
            let tilt = 1.0 + (band_idx as f32 / SPECTRUM_BANDS as f32) * 0.35;
            let level = (norm_val * tilt).clamp(0.0, 1.0);
            let scaled = (level * 10000.0) as u32;
            SPECTRUM_LEVELS[band_idx].store(scaled, Ordering::Relaxed);
        }

        if has_audio {
            record_audio_activity();
        }
    }
}

/// Reset all spectrum levels to zero.
pub fn reset_spectrum_levels() {
    for val in &SPECTRUM_LEVELS {
        val.store(0, Ordering::Relaxed);
    }
    if let Ok(mut guard) = RAW_SPECTRUM.lock() {
        *guard = [0u8; RAW_SPECTRUM_SIZE];
    }
}

/// Spectrum Analyzer that processes streaming audio samples.
pub struct SpectrumAnalyzer {
    r2c: Arc<dyn RealToComplex<f32>>,
    input_buf: Vec<f32>,
    fft_in: Vec<f32>,
    fft_out: Vec<Complex<f32>>,
    window: [f32; FFT_SIZE],
    band_bin_ranges: [(usize, usize); SPECTRUM_BANDS],
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(FFT_SIZE);
        let fft_in = r2c.make_input_vec();
        let fft_out = r2c.make_output_vec();

        // Precompute Hann window: w(n) = 0.5 * (1 - cos(2*pi*n / (N-1)))
        let mut window = [0.0f32; FFT_SIZE];
        let n_denom = (FFT_SIZE - 1) as f32;
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n_denom).cos());
        }

        // 16 Logarithmic frequency band boundaries from 25 Hz to 20,000 Hz
        let freq_bounds: [f32; SPECTRUM_BANDS + 1] = [
            25.0, 55.0, 95.0, 150.0, 240.0, 380.0, 600.0, 950.0,
            1500.0, 2400.0, 3800.0, 6000.0, 8500.0, 11500.0, 15000.0, 18500.0, 22050.0,
        ];

        let bin_res = sample_rate as f32 / FFT_SIZE as f32;
        let mut band_bin_ranges = [(0usize, 0usize); SPECTRUM_BANDS];
        for i in 0..SPECTRUM_BANDS {
            let start_bin = ((freq_bounds[i] / bin_res).floor() as usize).max(1);
            let end_bin = ((freq_bounds[i + 1] / bin_res).ceil() as usize).min(FFT_SIZE / 2);
            band_bin_ranges[i] = (start_bin, end_bin.max(start_bin));
        }

        Self {
            r2c,
            input_buf: Vec::with_capacity(FFT_SIZE * 2),
            fft_in,
            fft_out,
            window,
            band_bin_ranges,
        }
    }

    /// Feed mono audio samples into the analyzer.
    pub fn feed_samples(&mut self, samples: &[f32]) {
        self.input_buf.extend_from_slice(samples);

        while self.input_buf.len() >= FFT_SIZE {
            // Copy 512 samples and apply Hann window
            for i in 0..FFT_SIZE {
                self.fft_in[i] = self.input_buf[i] * self.window[i];
            }

            // Advance by hop size (e.g. 512 samples for clean block processing)
            self.input_buf.drain(..FFT_SIZE);

            // Execute Real-to-Complex FFT
            if self.r2c.process(&mut self.fft_in, &mut self.fft_out).is_ok() {
                let mut levels = [0.0f32; SPECTRUM_BANDS];

                for (band_idx, &(start_bin, end_bin)) in self.band_bin_ranges.iter().enumerate() {
                    let mut max_mag = 0.0f32;
                    for bin in start_bin..=end_bin {
                        if bin < self.fft_out.len() {
                            let c = self.fft_out[bin];
                            let mag = (c.re * c.re + c.im * c.im).sqrt();
                            if mag > max_mag {
                                max_mag = mag;
                            }
                        }
                    }

                    // Normalize magnitude relative to FFT size
                    let norm_mag = max_mag / (FFT_SIZE as f32 * 0.25);

                    // Dynamic range mapping (-55 dB to 0 dB mapped to 0.0 ..= 1.0)
                    let db = 20.0 * (norm_mag + 1e-5).log10();
                    let level = ((db + 55.0) / 55.0).clamp(0.0, 1.0);

                    // Slight perceptual tilt: boost high frequencies slightly for natural visual appearance
                    let tilt = 1.0 + (band_idx as f32 / SPECTRUM_BANDS as f32) * 0.35;
                    levels[band_idx] = (level * tilt).clamp(0.0, 1.0);
                }

                // Update global atomic levels
                for (i, &v) in levels.iter().enumerate() {
                    let scaled = (v * 10000.0) as u32;
                    SPECTRUM_LEVELS[i].store(scaled, Ordering::Relaxed);
                }

                // Compute and update raw 256-bin spectrum for visualizer
                let mut raw_bytes = [0u8; RAW_SPECTRUM_SIZE];
                for bin in 0..RAW_SPECTRUM_SIZE {
                    if bin < self.fft_out.len() {
                        let c = self.fft_out[bin];
                        let mag = (c.re * c.re + c.im * c.im).sqrt();
                        let norm_mag = mag / (FFT_SIZE as f32 * 0.25);
                        let db = 20.0 * (norm_mag + 1e-5).log10();
                        let level = ((db + 55.0) / 55.0).clamp(0.0, 1.0);
                        raw_bytes[bin] = (level * 255.0) as u8;
                    }
                }
                if let Ok(mut guard) = RAW_SPECTRUM.lock() {
                    *guard = raw_bytes;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapping::WindowKind;

    #[test]
    fn test_spectrum_consumer_property_matrix() {
        // Strict property check:
        // Main, MiniPlayer, and Visualizer display real-time audio spectrum.
        // Equalizer ONLY controls EQ sliders/presets and has ZERO spectrum widgets!
        assert!(WindowKind::Main.consumes_spectrum());
        assert!(WindowKind::MiniPlayer.consumes_spectrum());
        assert!(WindowKind::Visualizer.consumes_spectrum());
        assert!(!WindowKind::Equalizer.consumes_spectrum());
    }

    #[test]
    fn test_is_spectrum_needed_override_and_fast_path() {
        set_spectrum_needed_override(false);
        assert!(!is_spectrum_needed());

        // When false, process_post_dsp_pcm must drop samples immediately without panic or FFT
        let dummy_samples = vec![1000i16; 1024];
        process_post_dsp_pcm(&dummy_samples);

        set_spectrum_needed_override(true);
        assert!(is_spectrum_needed());

        // Restore clean state
        set_spectrum_needed_override(false);
    }

    #[test]
    fn test_audio_activity_tracking_and_30s_timeout() {
        let dummy_silence = vec![0i16; 512];
        process_post_dsp_pcm(&dummy_silence);

        // Feed non-silent samples
        let active_samples = vec![5000i16; 512];
        process_post_dsp_pcm(&active_samples);

        assert!(get_last_audio_active_ts() > 0);
        assert!(is_audio_active_30s());

        // Simulate time advancing 31 seconds
        let past = crate::controller::monotonic_now_ms().saturating_sub(31_000);
        LAST_AUDIO_ACTIVE_TS.store(past, Ordering::SeqCst);
        assert!(!is_audio_active_30s());

        // Call record_audio_activity to wake
        record_audio_activity();
        assert!(is_audio_active_30s());
    }

    #[test]
    fn test_update_raw_spectrum_from_engine_downsampling() {
        reset_spectrum_levels();
        let initial_levels = get_spectrum_levels();
        assert!(initial_levels.iter().all(|&v| v == 0.0));

        // Create a simulated 256-byte spectrum with a peak around bin 12 (~1 kHz)
        let mut simulated_raw = [0u8; RAW_SPECTRUM_SIZE];
        simulated_raw[12] = 200;

        update_raw_spectrum_from_engine(&simulated_raw);

        // Verify raw spectrum is stored for 3D visualizer
        let raw = get_raw_spectrum();
        assert_eq!(raw[12], 200);

        // Verify 16-band levels were downsampled for GUI meters
        let levels = get_spectrum_levels();
        // Bin 12 falls into band 6 ((6, 12)) or band 7 ((11, 18))
        assert!(levels[6] > 0.0 || levels[7] > 0.0, "Expected non-zero level in bands around bin 12");
        assert!(is_audio_active_30s());

        // Feeding silence should reset levels and not wake audio active
        reset_spectrum_levels();
        let silent_raw = [0u8; RAW_SPECTRUM_SIZE];
        update_raw_spectrum_from_engine(&silent_raw);
        let silent_levels = get_spectrum_levels();
        assert!(silent_levels.iter().all(|&v| v == 0.0));
    }
}
