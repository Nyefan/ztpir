#[derive(PartialEq, Debug, sqlx::Type)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    PendingConfirmation,
    Confirmed,
    UnsubscribedFromAll,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::PendingConfirmation => "pending_confirmation",
            SubscriptionStatus::Confirmed => "confirmed",
            SubscriptionStatus::UnsubscribedFromAll => "unsubscribed",
        }
    }
}

impl TryFrom<String> for SubscriptionStatus {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "pending_confirmation" => Ok(SubscriptionStatus::PendingConfirmation),
            "confirmed" => Ok(SubscriptionStatus::Confirmed),
            "unsubscribed" => Ok(SubscriptionStatus::UnsubscribedFromAll),
            _ => Err(format!("Unknown subscriber status: {}", s)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_err;
    use fake::{Fake, Faker};

    #[test]
    fn subscription_status_try_from_as_str_is_self() {
        for status in [
            SubscriptionStatus::PendingConfirmation,
            SubscriptionStatus::Confirmed,
            SubscriptionStatus::UnsubscribedFromAll,
        ] {
            assert_eq!(
                status,
                SubscriptionStatus::try_from(status.as_str().to_string()).unwrap()
            );
        }
    }

    #[test]
    fn subscription_status_from_invalid_str_is_err() {
        assert_err!(SubscriptionStatus::try_from(Faker.fake::<String>()));
    }
}
