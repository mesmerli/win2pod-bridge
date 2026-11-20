//! Low-latency ALAC packet decoder for AirPlay receiver / RAOP streaming.

use symphonia::core::codecs::{CodecParameters, Decoder, DecoderOptions, CODEC_TYPE_ALAC};
use symphonia::core::formats::Packet;
use thiserror::Error;

use crate::decoder::audio_buffer_to_i16;

/// Default 24-byte ALAC Magic Cookie for AirPlay (44.1kHz, 16-bit, 2-ch, 352 spf).
pub const DEFAULT_ALAC_MAGIC_COOKIE: [u8; 24] = [
    0x00, 0x00, 0x01, 0x60, // frame_length = 352 (0x0160)
    0x00,                   // compatible_version = 0
    0x10,                   // bit_depth = 16 (0x10)
    0x28,                   // pb = 40 (0x28)
    0x0a,                   // mb = 10 (0x0a)
    0x0e,                   // kb = 14 (0x0e)
    0x02,                   // num_channels = 2 (0x02)
    0x00, 0xff,             // max_run = 255 (0x00ff)
    0x00, 0x00, 0x00, 0x00, // max_frame_bytes = 0
    0x00, 0x00, 0x00, 0x00, // avg_bit_rate = 0
    0x00, 0x00, 0xac, 0x44, // sample_rate = 44100 (0xac44)
];

#[derive(Error, Debug)]
pub enum AlacDecodeError {
    #[error("Initialization error: {0}")]
    Init(String),
    #[error("Decode error: {0}")]
    Decode(String),
}

/// Standalone ALAC packet decoder.
pub struct AlacPacketDecoder {
    decoder: Box<dyn Decoder>,
    frames_per_packet: u32,
    channels: u8,
}

impl AlacPacketDecoder {
    /// Create a new ALAC packet decoder with standard AirPlay parameters (44.1kHz, 16-bit stereo, 352 spf).
    pub fn new() -> Result<Self, AlacDecodeError> {
        Self::with_cookie(&DEFAULT_ALAC_MAGIC_COOKIE)
    }

    /// Create an ALAC packet decoder with a specific 24-byte magic cookie.
    pub fn with_cookie(magic_cookie: &[u8]) -> Result<Self, AlacDecodeError> {
        let mut params = CodecParameters::new();
        params.for_codec(CODEC_TYPE_ALAC);
        params.with_extra_data(magic_cookie.to_vec().into_boxed_slice());

        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| AlacDecodeError::Init(e.to_string()))?;

        let frames_per_packet = if magic_cookie.len() >= 4 {
            u32::from_be_bytes([magic_cookie[0], magic_cookie[1], magic_cookie[2], magic_cookie[3]])
        } else {
            352
        };

        let channels = if magic_cookie.len() >= 10 {
            magic_cookie[9]
        } else {
            2
        };

        Ok(Self {
            decoder,
            frames_per_packet,
            channels,
        })
    }

    /// Decode an incoming ALAC payload into interleaved i16 PCM samples.
    ///
    /// If the packet is a RAOP initial/control packet (typically 32 bytes or empty),
    /// an empty Vec is returned without error.
    pub fn decode_packet(&mut self, payload: &[u8], timestamp: u64) -> Result<Vec<i16>, AlacDecodeError> {
        // AirPlay RAOP stream headers/silence info packets are 32 bytes
        if payload.len() <= 32 {
            return Ok(Vec::new());
        }

        let packet = Packet::new_from_slice(0, timestamp, self.frames_per_packet as u64, payload);

        let decoded = match self.decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                self.decoder.reset();
                return Ok(Vec::new());
            }
            Err(e) => return Err(AlacDecodeError::Decode(e.to_string())),
        };

        Ok(audio_buffer_to_i16(&decoded))
    }

    /// Reset decoder state (e.g. after stream flush or seek).
    pub fn reset(&mut self) {
        self.decoder.reset();
    }

    pub fn frames_per_packet(&self) -> u32 {
        self.frames_per_packet
    }

    pub fn channels(&self) -> u8 {
        self.channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::AlacEncoder;
    use airplay_core::{AudioCodec, AudioFormat, SampleRate};

    #[test]
    fn test_alac_roundtrip() {
        let format = AudioFormat {
            codec: AudioCodec::Alac,
            sample_rate: SampleRate::Hz44100,
            bit_depth: 16,
            channels: 2,
            frames_per_packet: 352,
        };

        let mut encoder = AlacEncoder::new(format).expect("Failed to create AlacEncoder");
        let cookie = encoder.magic_cookie();
        assert_eq!(cookie.len(), 24);

        let mut decoder = AlacPacketDecoder::with_cookie(&cookie).expect("Failed to create AlacPacketDecoder");

        // Generate 352 frames of test stereo audio (sawtooth + sine)
        let mut original_samples = Vec::with_capacity(352 * 2);
        for i in 0..352 {
            let left = ((i * 100) % 30000) as i16;
            let right = (-left) as i16;
            original_samples.push(left);
            original_samples.push(right);
        }

        // Encode
        let encoded_packet = crate::encoder::AudioEncoder::encode(&mut encoder, &original_samples)
            .expect("Encode failed");
        assert!(encoded_packet.data.len() > 32);

        // Decode
        let decoded_samples = decoder.decode_packet(&encoded_packet.data, 0)
            .expect("Decode failed");

        // ALAC is lossless: decoded samples must match original samples bit-for-bit
        assert_eq!(decoded_samples.len(), original_samples.len());
        assert_eq!(decoded_samples, original_samples);
    }
}
