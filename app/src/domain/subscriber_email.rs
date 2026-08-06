use anyhow::anyhow;
use std::str::FromStr;
use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl FromStr for SubscriberEmail {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.validate_email() {
            Ok(Self(s.to_string()))
        } else {
            Err(anyhow!("{s} is not a valid subscriber email."))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[test]
    fn at_least_one_valid_email_is_accepted() {
        let email = "ursula@ztpir.com".to_string();
        assert_ok!(email.parse::<SubscriberEmail>());
    }

    #[quickcheck_macros::quickcheck]
    fn many_valid_emails_are_accepted(email: ValidEmailFixture) -> bool {
        email.0.parse::<SubscriberEmail>().is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(email.parse::<SubscriberEmail>());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursulaztpir.com".to_string();
        assert_err!(email.parse::<SubscriberEmail>());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@ztpir.com".to_string();
        assert_err!(email.parse::<SubscriberEmail>());
    }
}
