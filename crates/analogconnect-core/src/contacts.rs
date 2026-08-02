use thiserror::Error;

/// A contact obtained from PBAP. Values are sensitive and must not be logged.
#[derive(Clone, PartialEq, Eq)]
pub struct Contact {
    pub display_name: Option<String>,
    pub phones: Vec<PhoneNumber>,
}

impl std::fmt::Debug for Contact {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Contact")
            .field(
                "display_name",
                &self.display_name.as_ref().map(|_| "[redacted]"),
            )
            .field("phone_count", &self.phones.len())
            .finish()
    }
}

/// A phone number with a presentation form and digits used for matching.
#[derive(Clone, PartialEq, Eq)]
pub struct PhoneNumber {
    pub display: String,
    pub normalized: String,
}

impl std::fmt::Debug for PhoneNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PhoneNumber([redacted])")
    }
}

impl PhoneNumber {
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
        if digits.len() < 3 {
            return None;
        }

        let international = trimmed.starts_with('+') || trimmed.starts_with("00");
        let normalized = if international {
            format!("+{}", digits.trim_start_matches('0'))
        } else {
            digits
        };

        Some(Self {
            display: trimmed.to_owned(),
            normalized,
        })
    }

    #[must_use]
    pub fn match_key(&self) -> &str {
        self.normalized.trim_start_matches('+')
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContactParseError {
    #[error("contact output contains a phone number without a contact header")]
    MissingContactHeader,
    #[error("contact output contains no usable contacts")]
    NoUsableContacts,
}

/// Parses the stable human output of `imsg contacts --raw`.
///
/// A block begins with a display name and contains indented phone-number lines.
/// The input is sensitive and must be discarded immediately after parsing.
pub fn parse_imsg_contacts(input: &str) -> Result<Vec<Contact>, ContactParseError> {
    let mut contacts = Vec::new();

    for block in input.replace("\r\n", "\n").split("\n\n") {
        let mut lines = block.lines().filter(|line| !line.trim().is_empty());
        let Some(header) = lines.next() else {
            continue;
        };
        if header.starts_with(char::is_whitespace) {
            return Err(ContactParseError::MissingContactHeader);
        }

        let display_name = match header.trim() {
            "(unknown)" => None,
            name => Some(name.to_owned()),
        };
        let phones = lines
            .filter(|line| line.starts_with(char::is_whitespace))
            .filter_map(PhoneNumber::parse)
            .collect::<Vec<_>>();

        if !phones.is_empty() {
            contacts.push(Contact {
                display_name,
                phones,
            });
        }
    }

    if contacts.is_empty() && !input.trim().is_empty() && input.trim() != "(no contacts)" {
        return Err(ContactParseError::NoUsableContacts);
    }
    Ok(contacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(parts: &[&str]) -> String {
        parts.concat()
    }

    #[test]
    fn parses_sanitized_imsg_blocks_without_debug_disclosure() {
        let first = number(&["+1", "202", "555", "0101"]);
        let second = number(&["020", "7946", "0018"]);
        let fixture = format!("Example Alpha\n  {first}\n\nExample Beta\n  {second}\n");

        let contacts = parse_imsg_contacts(&fixture).unwrap();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[0].display_name.as_deref(), Some("Example Alpha"));
        assert_eq!(contacts[0].phones[0].normalized, first);
        assert!(!format!("{:?}", contacts[0]).contains("Example Alpha"));
        assert!(!format!("{:?}", contacts[0]).contains(&first));
    }

    #[test]
    fn empty_phonebook_is_valid() {
        assert!(parse_imsg_contacts("(no contacts)\n").unwrap().is_empty());
    }

    #[test]
    fn rejects_unusable_nonempty_output() {
        assert_eq!(
            parse_imsg_contacts("unexpected output").unwrap_err(),
            ContactParseError::NoUsableContacts
        );
    }

    #[test]
    fn normalizes_punctuation_and_international_prefix() {
        let raw = number(&["00", "44", "20", "7946", "0018"]);
        let formatted = format!(
            "{} {} {} {}",
            &raw[0..4],
            &raw[4..6],
            &raw[6..10],
            &raw[10..]
        );
        let parsed = PhoneNumber::parse(&formatted).unwrap();
        assert!(parsed.normalized.starts_with('+'));
        assert_eq!(parsed.match_key(), raw.trim_start_matches('0'));
    }
}
