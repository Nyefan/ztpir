use anyhow::anyhow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

static FORBIDDEN_SUBSCRIBER_NAME_CHARACTERS: [char; 9] =
    ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
impl FromStr for SubscriberName {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let contains_forbidden_characters = s
            .chars()
            .any(|c| FORBIDDEN_SUBSCRIBER_NAME_CHARACTERS.contains(&c));
        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(anyhow!("{s} is not a valid subscriber name."))
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for SubscriberName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ë".repeat(256);
        assert_ok!(name.parse::<SubscriberName>());
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "ë".repeat(257);
        assert_err!(name.parse::<SubscriberName>());
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " \n".to_string();
        assert_err!(name.parse::<SubscriberName>());
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(name.parse::<SubscriberName>());
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &["/", "(", ")", "\"", "<", ">", "\\", "{", "}"] {
            assert_err!(name.parse::<SubscriberName>());
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(name.parse::<SubscriberName>());
    }
}
