//! Audio quality enhancement (Harmonic Exciter / HE high-frequency restoration).
//!
//! Uses `oximedia-effects` `ParametricEq` for pre-conditioning high frequencies
//! and `HarmonicExciter` to synthesize upper harmonics lost during lossy compression
//! (e.g. Spotify Free 160kbps, YouTube 128kbps, low-bitrate AAC/MP3).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use oximedia_effects::harmonic_exciter::{HarmonicExciter, HarmonicExciterConfig};
use oximedia_effects::parametric_eq::{EqBand, ParametricEq};

/// Sound enhancement mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[repr(u8)]
pub enum SoundEnhancerMode {
    #[default]
    Off = 0,
    /// Gentle high-frequency restoration, natural acoustic reproduction.
    Natural = 1,
    /// Dynamic restoration for lossy streams (Spotify Free 160k, YouTube).
    Dynamic = 2,
    /// Brilliant sparkle and rich presence for classical/instrumental.
    Brilliant = 3,
}

impl SoundEnhancerMode {
    pub const ALL: &'static [SoundEnhancerMode] = &[
        SoundEnhancerMode::Off,
        SoundEnhancerMode::Natural,
        SoundEnhancerMode::Dynamic,
        SoundEnhancerMode::Brilliant,
    ];

    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Natural,
            2 => Self::Dynamic,
            3 => Self::Brilliant,
            _ => Self::Off,
        }
    }
}

/// Shared parameters for the sound enhancer (lock-free updates from UI).
#[derive(Debug)]
pub struct SoundEnhancerParams {
    mode: AtomicU8,
}

impl SoundEnhancerParams {
    pub fn new(initial_mode: SoundEnhancerMode) -> Self {
        Self {
            mode: AtomicU8::new(initial_mode as u8),
        }
    }

    pub fn mode(&self) -> SoundEnhancerMode {
        SoundEnhancerMode::from_u8(self.mode.load(Ordering::Relaxed))
    }

    pub fn set_mode(&self, mode: SoundEnhancerMode) {
        self.mode.store(mode as u8, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.mode() != SoundEnhancerMode::Off
    }
}

impl Default for SoundEnhancerParams {
    fn default() -> Self {
        Self::new(SoundEnhancerMode::Off)
    }
}

/// Real-time audio enhancer module combining `ParametricEq` and `HarmonicExciter`.
pub struct SoundEnhancer {
    eq_l: ParametricEq,
    eq_r: ParametricEq,
    exciter_l: HarmonicExciter,
    exciter_r: HarmonicExciter,
    params: Arc<SoundEnhancerParams>,
    sample_rate: f32,
    cached_mode: SoundEnhancerMode,
}

impl SoundEnhancer {
    /// Create a new sound enhancer with shared parameters and sample rate.
    pub fn new(params: Arc<SoundEnhancerParams>, sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate as f32;
        let mode = params.mode();

        let (eq_l, eq_r, exciter_l, exciter_r) = Self::build_dsp(mode, sample_rate_f);

        Self {
            eq_l,
            eq_r,
            exciter_l,
            exciter_r,
            params,
            sample_rate: sample_rate_f,
            cached_mode: mode,
        }
    }

    /// Build internal DSP processors for a given mode.
    fn build_dsp(
        mode: SoundEnhancerMode,
        sample_rate: f32,
    ) -> (ParametricEq, ParametricEq, HarmonicExciter, HarmonicExciter) {
        let (drive, mix, hp_cutoff, shelf_freq, shelf_gain_db) = match mode {
            SoundEnhancerMode::Off => (0.0, 0.0, 6000.0, 8000.0, 0.0),
            SoundEnhancerMode::Natural => (0.28, 0.16, 6500.0, 8000.0, 1.5),
            SoundEnhancerMode::Dynamic => (0.42, 0.25, 5500.0, 7000.0, 2.5),
            SoundEnhancerMode::Brilliant => (0.58, 0.35, 4800.0, 6000.0, 3.5),
        };

        let mut eq_l = ParametricEq::new(sample_rate);
        let mut eq_r = ParametricEq::new(sample_rate);
        if shelf_gain_db != 0.0 {
            eq_l.add_band(EqBand::high_shelf(shelf_freq, shelf_gain_db));
            eq_r.add_band(EqBand::high_shelf(shelf_freq, shelf_gain_db));
        }

        let exciter_cfg = HarmonicExciterConfig {
            drive,
            mix,
            hp_cutoff_hz: hp_cutoff,
            sample_rate,
        };
        let exciter_l = HarmonicExciter::with_config(exciter_cfg.clone());
        let exciter_r = HarmonicExciter::with_config(exciter_cfg);

        (eq_l, eq_r, exciter_l, exciter_r)
    }

    /// Update internal processors when mode changes.
    fn update_mode(&mut self, new_mode: SoundEnhancerMode) {
        let (eq_l, eq_r, exciter_l, exciter_r) = Self::build_dsp(new_mode, self.sample_rate);
        self.eq_l = eq_l;
        self.eq_r = eq_r;
        self.exciter_l = exciter_l;
        self.exciter_r = exciter_r;
        self.cached_mode = new_mode;
    }

    /// Reset internal filter states (e.g. on stream start or seek).
    pub fn reset(&mut self) {
        self.eq_l.reset_states();
        self.eq_r.reset_states();
        self.exciter_l.reset();
        self.exciter_r.reset();
    }

    /// Process interleaved stereo 16-bit PCM samples in-place.
    pub fn process(&mut self, samples: &mut [i16]) {
        let mode = self.params.mode();
        if mode == SoundEnhancerMode::Off {
            return;
        }

        if self.cached_mode != mode {
            self.update_mode(mode);
        }

        for chunk in samples.chunks_exact_mut(2) {
            let left_in = chunk[0] as f32 / 32768.0;
            let right_in = chunk[1] as f32 / 32768.0;

            // 1. Pre-EQ high-shelf conditioning to restore presence band
            let mut left_buf = [left_in];
            let mut right_buf = [right_in];
            self.eq_l.process_samples(&mut left_buf);
            self.eq_r.process_samples(&mut right_buf);

            // 2. Harmonic Excitation (generates upper harmonics extending up to 20kHz+)
            let left_excited = self.exciter_l.process_sample(left_buf[0]);
            let right_excited = self.exciter_r.process_sample(right_buf[0]);

            // 3. Peak protection: transparent soft limiter to prevent digital clipping
            let left_out = peak_limit(left_excited);
            let right_out = peak_limit(right_excited);

            // 4. Convert back to 16-bit PCM
            chunk[0] = (left_out * 32767.0).clamp(-32768.0, 32767.0).round() as i16;
            chunk[1] = (right_out * 32767.0).clamp(-32768.0, 32767.0).round() as i16;
        }
    }

    /// Get reference to shared parameters.
    pub fn params(&self) -> &Arc<SoundEnhancerParams> {
        &self.params
    }
}

/// Transparent soft-limiting function to prevent harsh digital clipping.
///
/// Signal remains 100% linear within [-0.9, 0.9], and smoothly compresses
/// only when exceeding 0.9 to guarantee output never clips.
#[inline]
fn peak_limit(x: f32) -> f32 {
    if x > 0.9 {
        0.9 + 0.1 * ((x - 0.9) / 0.1).tanh()
    } else if x < -0.9 {
        -0.9 + 0.1 * ((x + 0.9) / 0.1).tanh()
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhancer_bypass_when_off() {
        let params = Arc::new(SoundEnhancerParams::new(SoundEnhancerMode::Off));
        let mut enhancer = SoundEnhancer::new(params.clone(), 44100);

        let mut samples = vec![1000i16, -2000i16, 3000i16, -4000i16];
        let original = samples.clone();
        enhancer.process(&mut samples);
        assert_eq!(samples, original);
    }

    #[test]
    fn test_enhancer_processes_audio_without_nan_or_inf() {
        let params = Arc::new(SoundEnhancerParams::new(SoundEnhancerMode::Dynamic));
        let mut enhancer = SoundEnhancer::new(params.clone(), 44100);

        let mut samples = vec![5000i16, -5000i16, 12000i16, -12000i16];
        enhancer.process(&mut samples);

        for &s in &samples {
            assert!(s != 0);
        }
    }

    #[test]
    fn test_enhancer_mode_switching() {
        let params = Arc::new(SoundEnhancerParams::new(SoundEnhancerMode::Off));
        let mut enhancer = SoundEnhancer::new(params.clone(), 44100);

        params.set_mode(SoundEnhancerMode::Natural);
        assert_eq!(params.mode(), SoundEnhancerMode::Natural);

        let mut samples = vec![1000i16, -1000i16];
        enhancer.process(&mut samples);
        assert_eq!(enhancer.cached_mode, SoundEnhancerMode::Natural);
    }

    #[test]
    fn test_peak_limiter() {
        assert_eq!(peak_limit(0.5), 0.5);
        assert_eq!(peak_limit(-0.5), -0.5);
        assert!(peak_limit(2.0) <= 1.0);
        assert!(peak_limit(-2.0) >= -1.0);
    }
}
