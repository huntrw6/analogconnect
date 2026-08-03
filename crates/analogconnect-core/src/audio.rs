use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub samples_per_channel: u16,
}

impl AudioFormat {
    pub const HFP_WIDEBAND: Self = Self {
        sample_rate_hz: 16_000,
        channels: 1,
        samples_per_channel: 120,
    };

    pub const HFP_NARROWBAND: Self = Self {
        sample_rate_hz: 8_000,
        channels: 1,
        samples_per_channel: 60,
    };

    #[must_use]
    pub fn frame_duration_micros(self) -> u64 {
        (u64::from(self.samples_per_channel) * 1_000_000) / u64::from(self.sample_rate_hz)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioFrame {
    sequence: u64,
    format: AudioFormat,
    samples: Vec<i16>,
}

impl AudioFrame {
    pub fn new(
        sequence: u64,
        format: AudioFormat,
        samples: Vec<i16>,
    ) -> Result<Self, AudioFrameError> {
        let expected = usize::from(format.samples_per_channel) * usize::from(format.channels);
        if format.sample_rate_hz == 0 || format.channels == 0 || samples.len() != expected {
            return Err(AudioFrameError::InvalidFormat);
        }
        Ok(Self {
            sequence,
            format,
            samples,
        })
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn format(&self) -> AudioFormat {
        self.format
    }

    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }
}

impl std::fmt::Debug for AudioFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioFrame")
            .field("sequence", &self.sequence)
            .field("format", &self.format)
            .field("sample_count", &self.samples.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AudioFrameError {
    #[error("audio frame does not match its declared format")]
    InvalidFormat,
}

const PACKET_MAGIC: &[u8; 4] = b"ACAP";
const PACKET_VERSION: u8 = 1;
const PACKET_HEADER_BYTES: usize = 24;

#[derive(Clone, PartialEq, Eq)]
pub struct AudioPacket {
    capture_time_micros: u64,
    frame: AudioFrame,
}

impl AudioPacket {
    #[must_use]
    pub const fn new(capture_time_micros: u64, frame: AudioFrame) -> Self {
        Self {
            capture_time_micros,
            frame,
        }
    }

    #[must_use]
    pub const fn capture_time_micros(&self) -> u64 {
        self.capture_time_micros
    }

    #[must_use]
    pub const fn frame(&self) -> &AudioFrame {
        &self.frame
    }

    pub fn encode(&self) -> Result<Vec<u8>, AudioPacketError> {
        let mut packet = Vec::with_capacity(PACKET_HEADER_BYTES + self.frame.samples.len() * 2);
        packet.extend_from_slice(PACKET_MAGIC);
        packet.push(PACKET_VERSION);
        let format = match self.frame.format {
            AudioFormat::HFP_NARROWBAND => 1,
            AudioFormat::HFP_WIDEBAND => 2,
            _ => return Err(AudioPacketError::UnsupportedFormat),
        };
        packet.push(format);
        packet.extend_from_slice(&[0, 0]);
        packet.extend_from_slice(&self.frame.sequence.to_be_bytes());
        packet.extend_from_slice(&self.capture_time_micros.to_be_bytes());
        for sample in &self.frame.samples {
            packet.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(packet)
    }

    pub fn decode(packet: &[u8]) -> Result<Self, AudioPacketError> {
        if packet.len() < PACKET_HEADER_BYTES
            || &packet[..4] != PACKET_MAGIC
            || packet[4] != PACKET_VERSION
            || packet[6] != 0
            || packet[7] != 0
        {
            return Err(AudioPacketError::InvalidHeader);
        }
        let format = match packet[5] {
            1 => AudioFormat::HFP_NARROWBAND,
            2 => AudioFormat::HFP_WIDEBAND,
            _ => return Err(AudioPacketError::UnsupportedFormat),
        };
        let payload = &packet[PACKET_HEADER_BYTES..];
        let expected_bytes = usize::from(format.samples_per_channel) * 2;
        if payload.len() != expected_bytes {
            return Err(AudioPacketError::InvalidPayload);
        }
        let sequence = u64::from_be_bytes(packet[8..16].try_into().unwrap());
        let capture_time_micros = u64::from_be_bytes(packet[16..24].try_into().unwrap());
        let samples = payload
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        let frame = AudioFrame::new(sequence, format, samples)
            .map_err(|_| AudioPacketError::InvalidPayload)?;
        Ok(Self::new(capture_time_micros, frame))
    }
}

impl std::fmt::Debug for AudioPacket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioPacket")
            .field("capture_time_micros", &self.capture_time_micros)
            .field("frame", &self.frame)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AudioPacketError {
    #[error("audio packet header is invalid")]
    InvalidHeader,
    #[error("audio packet format is unsupported")]
    UnsupportedFormat,
    #[error("audio packet payload length is invalid")]
    InvalidPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfp_frames_have_expected_duration() {
        assert_eq!(AudioFormat::HFP_WIDEBAND.frame_duration_micros(), 7_500);
        assert_eq!(AudioFormat::HFP_NARROWBAND.frame_duration_micros(), 7_500);
    }

    #[test]
    fn debug_never_contains_sample_values() {
        let samples = vec![12_345; usize::from(AudioFormat::HFP_WIDEBAND.samples_per_channel)];
        let frame = AudioFrame::new(7, AudioFormat::HFP_WIDEBAND, samples).unwrap();
        let debug = format!("{frame:?}");
        assert!(!debug.contains("12345"));
        assert!(debug.contains("sample_count"));
    }

    #[test]
    fn packet_codec_round_trips_without_debug_disclosure() {
        let samples = (0..AudioFormat::HFP_WIDEBAND.samples_per_channel)
            .map(|value| i16::try_from(value).unwrap() - 60)
            .collect();
        let frame = AudioFrame::new(42, AudioFormat::HFP_WIDEBAND, samples).unwrap();
        let packet = AudioPacket::new(99_000, frame);
        let encoded = packet.encode().unwrap();
        let decoded = AudioPacket::decode(&encoded).unwrap();
        assert_eq!(decoded, packet);
        assert!(!format!("{decoded:?}").contains("-60"));
    }

    #[test]
    fn packet_codec_rejects_malformed_and_unknown_data() {
        assert_eq!(
            AudioPacket::decode(b"short"),
            Err(AudioPacketError::InvalidHeader)
        );
        let frame = AudioFrame::new(
            1,
            AudioFormat::HFP_NARROWBAND,
            vec![0; usize::from(AudioFormat::HFP_NARROWBAND.samples_per_channel)],
        )
        .unwrap();
        let mut encoded = AudioPacket::new(0, frame).encode().unwrap();
        encoded[5] = 99;
        assert_eq!(
            AudioPacket::decode(&encoded),
            Err(AudioPacketError::UnsupportedFormat)
        );
        encoded[5] = 1;
        encoded.pop();
        assert_eq!(
            AudioPacket::decode(&encoded),
            Err(AudioPacketError::InvalidPayload)
        );
    }
}
