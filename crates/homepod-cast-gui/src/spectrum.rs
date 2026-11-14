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

/// Process post-DSP audio samples (interleaved stereo i16) to update real-time spectrum and peak level.
pub fn process_post_dsp_pcm(samples: &[i16]) {
    if samples.is_empty() || !crate::tray::is_any_window_visible() {
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
    if is_active && !prev {
        // Audio transitioned from silence to active: wake UI immediately for zero-latency animation
        crate::gui::request_ui_repaint();
    }

    if max_abs < 0.0001 {
        WAS_AUDIO_ACTIVE.store(false, Ordering::Relaxed);
        reset_spectrum_levels();
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
