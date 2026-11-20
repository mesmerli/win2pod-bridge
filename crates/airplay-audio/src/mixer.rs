//! Software Audio Mixer with Soft-Limiter and Zero-Latency Direct Passthrough.
//!
//! Blends primary audio (e.g. PC system audio) and secondary audio (e.g. iPhone AirPlay stream)
//! into a single unified stream for HomePod / AirPlay 2 output.
//!
//! Pacing Architecture:
//! - When only secondary (iPhone) is active: incoming AirPlay packets are forwarded directly to
//!   the HomePod output queue with zero buffering and zero clock drift, maintaining pristine
//!   8ms cadence. Any dummy silence frames from WASAPI loopback timeout are cleanly dropped.
//! - When only primary (PC) is active: PC capture frames pass directly to HomePod.
//! - When both primary and secondary are active: secondary samples are queued into a low-latency
//!   jitter buffer, smoothly mixed into primary frames using Pade-approximated tanh soft-limiting,
//!   and streamed to HomePod without digital clipping or packet collisions.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::live_decoder::{LiveFrameSender, LivePcmFrame};

/// Soft-limiting function using fast tanh approximation to prevent digital clipping.
/// Maps (-inf, +inf) smoothly into (-1.0, 1.0).
#[inline]
pub fn soft_limit(sample: f32) -> f32 {
    if sample <= -3.0 {
        -1.0
    } else if sample >= 3.0 {
        1.0
    } else {
        // Pade approximation of tanh(x)
        let x2 = sample * sample;
        sample * (27.0 + x2) / (27.0 + 9.0 * x2)
    }
}

/// Software Audio Mixer for combining PC and AirPlay audio streams.
pub struct SoftwareAudioMixer {
    output: LiveFrameSender,
    secondary_queue: Arc<Mutex<VecDeque<i16>>>,
    last_active_primary: Arc<Mutex<Option<Instant>>>,
    last_secondary: Arc<Mutex<Option<Instant>>>,
    sample_rate: u32,
    channels: u8,
}

impl SoftwareAudioMixer {
    /// Create a new mixer that sends mixed frames to `output`.
    pub fn new(output: LiveFrameSender, sample_rate: u32, channels: u8) -> Self {
        Self {
            output,
            secondary_queue: Arc::new(Mutex::new(VecDeque::with_capacity(8192))),
            last_active_primary: Arc::new(Mutex::new(None)),
            last_secondary: Arc::new(Mutex::new(None)),
            sample_rate,
            channels,
        }
    }

    /// Check if primary (PC) audio is currently actively playing sound (non-silent).
    #[inline]
    pub fn is_primary_active(&self) -> bool {
        let guard = self.last_active_primary.lock().unwrap();
        guard.map_or(false, |t| t.elapsed() < Duration::from_millis(150))
    }

    /// Check if secondary (AirPlay iPhone) audio is currently streaming.
    #[inline]
    pub fn is_secondary_active(&self) -> bool {
        let guard = self.last_secondary.lock().unwrap();
        guard.map_or(false, |t| t.elapsed() < Duration::from_millis(200))
    }

    /// Process a primary audio frame (e.g. from PC loopback capture).
    /// If secondary audio is present in the queue, mixes it in with soft-limiting.
    pub fn process_primary(&self, mut frame: LivePcmFrame) -> bool {
        let frame_has_audio = frame.samples.iter().any(|&s| s.abs() > 32);
        if frame_has_audio {
            let mut last_pri = self.last_active_primary.lock().unwrap();
            *last_pri = Some(Instant::now());
        }

        let secondary_active = self.is_secondary_active();

        // If secondary (iPhone) is actively streaming and PC primary is silent:
        // Drop the 50ms dummy silence keepalive from WASAPI so it does not inject dead silence holes
        // into the real-time iPhone audio stream!
        if secondary_active && !frame_has_audio {
            return true;
        }

        // If both are active, mix secondary samples into primary frame with soft limiter
        if secondary_active && frame_has_audio {
            let mut queue = self.secondary_queue.lock().unwrap();
            if !queue.is_empty() {
                for sample in frame.samples.iter_mut() {
                    if let Some(sec_sample) = queue.pop_front() {
                        let s1 = *sample as f32 / 32768.0;
                        let s2 = sec_sample as f32 / 32768.0;
                        let sum = s1 + s2;
                        let mixed = soft_limit(sum);
                        *sample = (mixed * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    } else {
                        break;
                    }
                }
            }
        }

        self.output.try_send(frame)
    }

    /// Push secondary audio samples (e.g. from iPhone AirPlay Receiver).
    pub fn push_secondary(&self, samples: &[i16]) {
        {
            let mut last_sec = self.last_secondary.lock().unwrap();
            *last_sec = Some(Instant::now());
        }

        let primary_active = self.is_primary_active();
        let packet_size = 352 * self.channels as usize;

        let mut queue = self.secondary_queue.lock().unwrap();
        // Prevent queue from growing unbounded if streaming stalls (max 1 second)
        let max_queue = 44100 * self.channels as usize;
        let cur_len = queue.len();
        if cur_len + samples.len() > max_queue {
            let excess = (cur_len + samples.len()) - max_queue;
            queue.drain(..excess.min(cur_len));
        }
        queue.extend(samples);

        if !primary_active {
            if queue.len() < packet_size {
                let chunk: Vec<i16> = queue.drain(..).collect();
                let _ = self.output.try_send(LivePcmFrame {
                    samples: chunk,
                    channels: self.channels,
                    sample_rate: self.sample_rate,
                });
            } else {
                while queue.len() >= packet_size {
                    let chunk: Vec<i16> = queue.drain(..packet_size).collect();
                    let _ = self.output.try_send(LivePcmFrame {
                        samples: chunk,
                        channels: self.channels,
                        sample_rate: self.sample_rate,
                    });
                }
            }
        }
    }

    /// Clear secondary audio queue (e.g. on stream pause or flush).
    pub fn clear_secondary(&self) {
        let mut queue = self.secondary_queue.lock().unwrap();
        queue.clear();
        let mut last_sec = self.last_secondary.lock().unwrap();
        *last_sec = None;
    }
}

impl Drop for SoftwareAudioMixer {
    fn drop(&mut self) {
        self.clear_secondary();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_decoder::LiveAudioDecoder;

    #[test]
    fn test_soft_limit_boundaries() {
        assert!((soft_limit(0.0) - 0.0).abs() < 1e-4);
        assert!(soft_limit(5.0) <= 1.0);
        assert!(soft_limit(-5.0) >= -1.0);
        // Monotonic
        assert!(soft_limit(1.5) > soft_limit(1.0));
        assert!(soft_limit(0.5) > soft_limit(0.0));
    }

    #[test]
    fn test_mixing_both_active() {
        let (sender, mut decoder) = LiveAudioDecoder::create_pair(44100, 2, 16);
        let mixer = SoftwareAudioMixer::new(sender, 44100, 2);

        // First establish that primary is actively playing sound
        mixer.process_primary(LivePcmFrame {
            samples: vec![1000, 1000],
            channels: 2,
            sample_rate: 44100,
        });
        let _ = decoder.decode_frame();

        // Push secondary audio: 4 samples while primary is active
        mixer.push_secondary(&[10000, 10000, 10000, 10000]);

        // Process primary audio: 4 samples
        let frame = LivePcmFrame {
            samples: vec![10000, 10000, 10000, 10000],
            channels: 2,
            sample_rate: 44100,
        };

        mixer.process_primary(frame);

        // Verify mixed frame received via decoder
        let decoded = decoder.decode_frame().expect("Decode failed").expect("Frame expected");
        assert_eq!(decoded.samples.len(), 4);
        // Mixed samples should be greater than original 10000
        for s in decoded.samples {
            assert!(s > 10000);
        }
    }

    #[test]
    fn test_secondary_only_direct_passthrough() {
        let (sender, mut decoder) = LiveAudioDecoder::create_pair(44100, 2, 16);
        let mixer = SoftwareAudioMixer::new(sender, 44100, 2);

        // Primary is silent / not active. Push secondary samples.
        mixer.push_secondary(&[500, 600, 700, 800]);

        let decoded = decoder.decode_frame().expect("Decode failed").expect("Frame expected");
        assert_eq!(decoded.samples, vec![500, 600, 700, 800]);
    }

    #[test]
    fn test_primary_dummy_silence_dropped_when_secondary_active() {
        let (sender, mut decoder) = LiveAudioDecoder::create_pair(44100, 2, 16);
        let mixer = SoftwareAudioMixer::new(sender, 44100, 2);

        // Push secondary audio
        mixer.push_secondary(&[123, 456]);

        // Process a dummy silence frame from WASAPI loopback timeout
        let silence_frame = LivePcmFrame {
            samples: vec![0, 0, 0, 0],
            channels: 2,
            sample_rate: 44100,
        };
        mixer.process_primary(silence_frame);

        // The decoder should only have received the secondary audio, NOT the dummy silence!
        let decoded = decoder.decode_frame().expect("Decode failed").expect("Frame expected");
        assert_eq!(decoded.samples, vec![123, 456]);

        // No more frames should be in the queue (silence was dropped)
        decoder.set_recv_timeout(Duration::from_millis(5));
        let next = decoder.decode_frame().unwrap();
        assert!(next.is_none());
    }
}
