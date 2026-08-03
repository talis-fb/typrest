use std::fmt;

use thiserror::Error;

/// Canonical route-like identifier of a template available to Typrest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateId(String);

impl TemplateId {
    /// Creates a template ID after validating every path segment.
    pub fn new(value: impl Into<String>) -> Result<Self, TemplateIdError> {
        let value = value.into();
        validate(&value)?;

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

impl AsRef<str> for TemplateId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for TemplateId {
    type Error = TemplateIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for TemplateId {
    type Error = TemplateIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TemplateIdError {
    #[error("template ID cannot be empty")]
    Empty,

    #[error("template ID contains an empty path segment at position {segment}")]
    EmptySegment { segment: usize },

    #[error("template ID segment '{value}' is reserved because it starts with '_'")]
    ReservedSegment { value: String },

    #[error(
        "template ID segment '{segment}' contains invalid character '{character}' at position {position}; only ASCII alphanumeric characters, '-' and '_' are allowed"
    )]
    InvalidCharacter {
        segment: String,
        character: char,
        position: usize,
    },
}

fn validate(value: &str) -> Result<(), TemplateIdError> {
    if value.is_empty() {
        return Err(TemplateIdError::Empty);
    }

    for (segment_index, segment) in value.split('/').enumerate() {
        if segment.is_empty() {
            return Err(TemplateIdError::EmptySegment {
                segment: segment_index + 1,
            });
        }
        if segment.starts_with('_') {
            return Err(TemplateIdError::ReservedSegment {
                value: segment.to_owned(),
            });
        }

        for (position, character) in segment.chars().enumerate() {
            if !character.is_ascii_alphanumeric() && character != '-' && character != '_' {
                return Err(TemplateIdError::InvalidCharacter {
                    segment: segment.to_owned(),
                    character,
                    position: position + 1,
                });
            }
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{TemplateId, TemplateIdError};

    #[test]
    fn accepts_valid_template_ids() {
        for value in [
            "invoice",
            "invoice-v2",
            "invoice_v2",
            "Invoice2",
            "-invoice",
            "financial/invoice_v2",
        ] {
            let name = TemplateId::new(value).expect("template ID should be valid");

            assert_eq!(name.as_str(), value);
        }
    }

    #[test]
    fn rejects_empty_template_id() {
        assert_eq!(TemplateId::new(""), Err(TemplateIdError::Empty));
    }

    #[test]
    fn rejects_unsupported_characters() {
        assert_eq!(
            TemplateId::new(".invoice"),
            Err(TemplateIdError::InvalidCharacter {
                segment: ".invoice".to_owned(),
                character: '.',
                position: 1,
            })
        );

        assert_eq!(
            TemplateId::new("invoice v2"),
            Err(TemplateIdError::InvalidCharacter {
                segment: "invoice v2".to_owned(),
                character: ' ',
                position: 8,
            })
        );
        assert_eq!(
            TemplateId::new("invoiceλ"),
            Err(TemplateIdError::InvalidCharacter {
                segment: "invoiceλ".to_owned(),
                character: 'λ',
                position: 8,
            })
        );
        assert!(matches!(
            TemplateId::new("financial/../invoice"),
            Err(TemplateIdError::InvalidCharacter { .. })
        ));
    }

    #[test]
    fn rejects_empty_and_reserved_segments() {
        for value in ["/invoice", "invoice/", "invoice//v2"] {
            assert!(matches!(
                TemplateId::new(value),
                Err(TemplateIdError::EmptySegment { .. })
            ));
        }

        assert_eq!(
            TemplateId::new("financial/_draft"),
            Err(TemplateIdError::ReservedSegment {
                value: "_draft".to_owned()
            })
        );
    }

    #[test]
    fn exposes_validated_segments() {
        let id = TemplateId::new("financial/invoice").unwrap();

        assert_eq!(id.segments().collect::<Vec<_>>(), ["financial", "invoice"]);
    }
}
