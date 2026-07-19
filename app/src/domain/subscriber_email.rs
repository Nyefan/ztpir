pub struct SubscriberEmail(String);

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
