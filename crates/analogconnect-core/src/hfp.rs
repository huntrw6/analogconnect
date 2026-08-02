use thiserror::Error;

#[derive(Clone, PartialEq, Eq)]
pub struct DialTarget(String);

impl DialTarget {
    pub fn parse(input: &str) -> Result<Self, HfpArgumentError> {
        let target = input.trim();
        if target.is_empty()
            || target.len() > 32
            || !target
                .chars()
                .all(|character| character.is_ascii_digit() || matches!(character, '+' | '*' | '#'))
        {
            return Err(HfpArgumentError::InvalidDialTarget);
        }
        Ok(Self(target.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for DialTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DialTarget([redacted])")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DtmfTone(char);

impl DtmfTone {
    pub fn parse(input: char) -> Result<Self, HfpArgumentError> {
        let tone = input.to_ascii_uppercase();
        if tone.is_ascii_digit() || matches!(tone, '*' | '#' | 'A'..='D') {
            Ok(Self(tone))
        } else {
            Err(HfpArgumentError::InvalidDtmfTone)
        }
    }

    #[must_use]
    pub const fn value(self) -> char {
        self.0
    }
}

impl std::fmt::Debug for DtmfTone {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DtmfTone([redacted])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gain(u8);

impl Gain {
    pub fn new(value: u8) -> Result<Self, HfpArgumentError> {
        if value <= 15 {
            Ok(Self(value))
        } else {
            Err(HfpArgumentError::InvalidGain)
        }
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum CallCommand {
    Answer,
    Reject,
    HangUp,
    Dial(DialTarget),
    SendDtmf(DtmfTone),
    SetMicrophoneMuted(bool),
    SetSpeakerGain(Gain),
    SetMicrophoneGain(Gain),
}

impl std::fmt::Debug for CallCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Answer => formatter.write_str("Answer"),
            Self::Reject => formatter.write_str("Reject"),
            Self::HangUp => formatter.write_str("HangUp"),
            Self::Dial(_) => formatter.write_str("Dial([redacted])"),
            Self::SendDtmf(_) => formatter.write_str("SendDtmf([redacted])"),
            Self::SetMicrophoneMuted(muted) => formatter
                .debug_tuple("SetMicrophoneMuted")
                .field(muted)
                .finish(),
            Self::SetSpeakerGain(gain) => {
                formatter.debug_tuple("SetSpeakerGain").field(gain).finish()
            }
            Self::SetMicrophoneGain(gain) => formatter
                .debug_tuple("SetMicrophoneGain")
                .field(gain)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HfpArgumentError {
    #[error("invalid dial target")]
    InvalidDialTarget,
    #[error("invalid DTMF tone")]
    InvalidDtmfTone,
    #[error("gain must be in the HFP range 0 through 15")]
    InvalidGain,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_commands_have_redacted_debug_output() {
        let target = DialTarget::parse(&["+1", "202", "555", "0101"].concat()).unwrap();
        let command = CallCommand::Dial(target);
        let debug = format!("{command:?}");
        assert_eq!(debug, "Dial([redacted])");
        assert!(!debug.chars().any(|character| character.is_ascii_digit()));

        let dtmf = CallCommand::SendDtmf(DtmfTone::parse('7').unwrap());
        assert_eq!(format!("{dtmf:?}"), "SendDtmf([redacted])");
    }

    #[test]
    fn validates_hfp_argument_domains() {
        assert!(DialTarget::parse("").is_err());
        assert!(DialTarget::parse("not a dial string").is_err());
        assert!(DtmfTone::parse('D').is_ok());
        assert!(DtmfTone::parse('z').is_err());
        assert_eq!(Gain::new(15).unwrap().value(), 15);
        assert!(Gain::new(16).is_err());
    }
}
