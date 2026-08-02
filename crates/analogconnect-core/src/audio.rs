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
}
