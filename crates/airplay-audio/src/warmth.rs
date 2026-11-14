//! Audio warmth enhancement (Warmth Enhancer / WE).
//!
//! Emulates analog turntable and vacuum tube/tape acoustic characteristics:
//! 1. Low-frequency shelf EQ (50Hz - 100Hz, +1.5dB to +2.0dB) to simulate tonearm
//!    and platter physical resonance for a fatter, warmer bass response.
//! 2. Subtle analog saturation (2% - 5% drive via `oximedia-effects`) to eliminate
//!    digital cold/harsh dryness and provide vintage vinyl musical texture.
//! 3. Soft-knee peak limiting to guarantee transparent headroom without clipping.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use oximedia_effects::parametric_eq::{EqBand, ParametricEq};
use oximedia_effects::saturation::{SaturationConfig, SaturationType, Saturator};

/// Warmth enhancement mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[repr(u8)]
pub enum WarmthEnhancerMode {
    #[default]
    Off = 0,
    /// Subtle warmth: 2.5% drive, +1.5dB low-shelf @ 60Hz.
    Subtle = 1,
    /// Classic warm vinyl: 3.8% drive, +1.8dB low-shelf @ 75Hz.
    Warm = 2,
    /// Vintage rich tube/vinyl: 5.0% drive, +2.0dB low-shelf @ 90Hz.
    Vintage = 3,
}

impl WarmthEnhancerMode {
    pub const ALL: &'static [WarmthEnhancerMode] = &[
        WarmthEnhancerMode::Off,
        WarmthEnhancerMode::Subtle,
        WarmthEnhancerMode::Warm,
        WarmthEnhancerMode::Vintage,
    ];

    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Subtle,
            2 => Self::Warm,
            3 => Self::Vintage,
            _ => Self::Off,
        }
    }
}

/// Shared parameters for the warmth enhancer (lock-free updates from UI).
#[derive(Debug)]
pub struct WarmthEnhancerParams {
    mode: AtomicU8,
}

impl WarmthEnhancerParams {
    pub fn new(initial_mode: WarmthEnhancerMode) -> Self {
        Self {
            mode: AtomicU8::new(initial_mode as u8),
        }
    }

    pub fn mode(&self) -> WarmthEnhancerMode {
        WarmthEnhancerMode::from_u8(self.mode.load(Ordering::Relaxed))
    }

    pub fn set_mode(&self, mode: WarmthEnhancerMode) {
        self.mode.store(mode as u8, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.mode() != WarmthEnhancerMode::Off
    }
}

impl Default for WarmthEnhancerParams {
    fn default() -> Self {
        Self::new(WarmthEnhancerMode::Off)
    }
}

/// Real-time audio warmth processor combining low-shelf physical resonance and analog saturation.
pub struct WarmthEnhancer {
    eq_l: ParametricEq,
    eq_r: ParametricEq,
    sat_l: Saturator,
    sat_r: Saturator,
    params: Arc<WarmthEnhancerParams>,
    sample_rate: f32,
    cached_mode: WarmthEnhancerMode,
}

impl WarmthEnhancer {
    /// Create a new warmth enhancer with shared parameters and sample rate.
    pub fn new(params: Arc<WarmthEnhancerParams>, sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate as f32;
        let mode = params.mode();

        let (eq_l, eq_r, sat_l, sat_r) = Self::build_dsp(mode, sample_rate_f);

        Self {
            eq_l,
            eq_r,
            sat_l,
            sat_r,
            params,
            sample_rate: sample_rate_f,
            cached_mode: mode,
        }
    }

    /// Build internal DSP processors for a given mode.
    fn build_dsp(
        mode: WarmthEnhancerMode,
        sample_rate: f32,
    ) -> (ParametricEq, ParametricEq, Saturator, Saturator) {
        let (sat_type, drive, tone, shelf_freq, shelf_gain_db) = match mode {
            WarmthEnhancerMode::Off => (SaturationType::Warmth, 0.0, 1.0, 60.0, 0.0),
            WarmthEnhancerMode::Subtle => (SaturationType::Tape, 0.025, 0.98, 60.0, 1.5),
            WarmthEnhancerMode::Warm => (SaturationType::Tape, 0.038, 0.95, 75.0, 1.8),
            WarmthEnhancerMode::Vintage => (SaturationType::Tube, 0.050, 0.92, 90.0, 2.0),
        };

        let mut eq_l = ParametricEq::new(sample_rate);
        let mut eq_r = ParametricEq::new(sample_rate);
        if shelf_gain_db != 0.0 {
            eq_l.add_band(EqBand::low_shelf(shelf_freq, shelf_gain_db));
            eq_r.add_band(EqBand::low_shelf(shelf_freq, shelf_gain_db));
        }

        let sat_cfg = SaturationConfig {
            sat_type,
            drive,
            tone,
            mix: if mode == WarmthEnhancerMode::Off { 0.0 } else { 1.0 },
        };
        let sat_l = Saturator::new(sat_cfg, sample_rate);
        let sat_r = Saturator::new(sat_cfg, sample_rate);

        (eq_l, eq_r, sat_l, sat_r)
    }

    /// Update internal processors when mode changes.
    fn update_mode(&mut self, new_mode: WarmthEnhancerMode) {
        let (eq_l, eq_r, sat_l, sat_r) = Self::build_dsp(new_mode, self.sample_rate);
        self.eq_l = eq_l;
        self.eq_r = eq_r;
        self.sat_l = sat_l;
        self.sat_r = sat_r;
        self.cached_mode = new_mode;
    }

    /// Reset internal filter states (e.g. on stream start or seek).
    pub fn reset(&mut self) {
        self.eq_l.reset_states();
        self.eq_r.reset_states();
        self.sat_l.reset();
        self.sat_r.reset();
    }

    /// Process interleaved stereo 16-bit PCM samples in-place.
    pub fn process(&mut self, samples: &mut [i16]) {
        let mode = self.params.mode();
        if mode == WarmthEnhancerMode::Off {
            return;
        }

        if self.cached_mode != mode {
            self.update_mode(mode);
        }

        for chunk in samples.chunks_exact_mut(2) {
            let left_in = chunk[0] as f32 / 32768.0;
            let right_in = chunk[1] as f32 / 32768.0;

            // 1. Pre-EQ low-shelf conditioning for tonearm/platter resonance
            let mut left_buf = [left_in];
            let mut right_buf = [right_in];
            self.eq_l.process_samples(&mut left_buf);
            self.eq_r.process_samples(&mut right_buf);

            // 2. Analog tape/tube saturation for harmonic warmth
            let left_sat = self.sat_l.process_sample(left_buf[0]);
            let right_sat = self.sat_r.process_sample(right_buf[0]);

            // 3. Peak protection: transparent soft limiter
            let left_out = peak_limit(left_sat);
            let right_out = peak_limit(right_sat);

            // 4. Convert back to 16-bit PCM
            chunk[0] = (left_out * 32767.0).clamp(-32768.0, 32767.0).round() as i16;
            chunk[1] = (right_out * 32767.0).clamp(-32768.0, 32767.0).round() as i16;
        }
    }

    /// Get reference to shared parameters.
    pub fn params(&self) -> &Arc<WarmthEnhancerParams> {
        &self.params
    }
}

/// Transparent soft-limiting function to prevent harsh digital clipping.
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
    fn test_warmth_enhancer_bypass_when_off() {
        let params = Arc::new(WarmthEnhancerParams::new(WarmthEnhancerMode::Off));
        let mut warmth = WarmthEnhancer::new(params.clone(), 44100);

        let mut samples = vec![1000i16, -2000i16, 3000i16, -4000i16];
        let original = samples.clone();
        warmth.process(&mut samples);

        assert_eq!(samples, original);
    }

    #[test]
    fn test_warmth_enhancer_processes_audio() {
        let params = Arc::new(WarmthEnhancerParams::new(WarmthEnhancerMode::Warm));
        let mut warmth = WarmthEnhancer::new(params.clone(), 44100);

        // Feed sine wave
        let mut samples = Vec::new();
        for i in 0..1000 {
            let sample = (f32::sin(i as f32 * 0.05) * 15000.0) as i16;
            samples.push(sample);
            samples.push(sample);
        }

        let original = samples.clone();
        warmth.process(&mut samples);

        assert_ne!(samples, original);
        assert!(!samples.is_empty());
    }

    #[test]
    fn test_warmth_enhancer_mode_switching() {
        let params = Arc::new(WarmthEnhancerParams::new(WarmthEnhancerMode::Off));
        let _warmth = WarmthEnhancer::new(params.clone(), 44100);

        assert!(!params.is_enabled());

        params.set_mode(WarmthEnhancerMode::Subtle);
        assert!(params.is_enabled());
        assert_eq!(params.mode(), WarmthEnhancerMode::Subtle);

        params.set_mode(WarmthEnhancerMode::Warm);
        assert_eq!(params.mode(), WarmthEnhancerMode::Warm);

        params.set_mode(WarmthEnhancerMode::Vintage);
        assert_eq!(params.mode(), WarmthEnhancerMode::Vintage);
    }

    #[test]
    fn test_warmth_enhancer_silence_remains_silence() {
        let params = Arc::new(WarmthEnhancerParams::new(WarmthEnhancerMode::Vintage));
        let mut warmth = WarmthEnhancer::new(params.clone(), 44100);

        let mut samples = vec![0i16; 512];
        warmth.process(&mut samples);

        for &s in &samples {
            assert_eq!(s, 0);
        }
    }
}
