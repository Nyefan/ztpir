use unicode_segmentation::UnicodeSegmentation;

pub struct SubscriberName(String);
pub struct SubscriberEmail(String);

pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}

static FORBIDDEN_SUBSCRIBER_NAME_CHARACTERS: [char; 9] =
    ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let contains_forbidden_characters = s
            .chars()
            .any(|c| FORBIDDEN_SUBSCRIBER_NAME_CHARACTERS.contains(&c));
        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        if is_empty_or_whitespace {
            Err(format!("{} is not a valid subscriber email.", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
