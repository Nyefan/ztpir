use super::SubscriberEmail;
use super::SubscriberName;
use uuid::Uuid;

pub type SubscriberConfirmationToken = String;
pub type SubscriberId = Uuid;

pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
    pub confirmation_token: SubscriberConfirmationToken,
}
