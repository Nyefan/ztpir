use crate::domain::SubscriberEmail;
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

pub struct EmailClient {
    http_client: reqwest::Client,
    base_url: reqwest::Url,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: reqwest::Url,
        sender: SubscriberEmail,
        authorization_token: SecretString,
    ) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(), //grrrrr - this book really hates results
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        let url = self
            .base_url
            .join("/email")
            .map_err(|err| err.to_string())?;
        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&body)
            .send()
            .await
            .map_err(|err| err.to_string())?
            .error_for_status()
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::{ExposeSecret, SecretString};
    use std::time::Duration;
    use wiremock::matchers::{any, header, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            serde_json::from_slice(&request.body).is_ok_and(|body: serde_json::Value| {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            })
        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token = SecretString::from(Faker.fake::<String>());
        let email_client = EmailClient::new(
            reqwest::Url::parse(&mock_server.uri()).unwrap(),
            sender,
            authorization_token.clone(),
        );

        Mock::given(method("POST"))
            .and(header(
                "X-Postmark-Server-Token",
                authorization_token.expose_secret(),
            ))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token = SecretString::from(Faker.fake::<String>());
        let email_client = EmailClient::new(
            reqwest::Url::parse(&mock_server.uri()).unwrap(),
            sender,
            authorization_token,
        );

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(result);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token = SecretString::from(Faker.fake::<String>());
        let email_client = EmailClient::new(
            reqwest::Url::parse(&mock_server.uri()).unwrap(),
            sender,
            authorization_token,
        );

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(result);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token = SecretString::from(Faker.fake::<String>());
        let email_client = EmailClient::new(
            reqwest::Url::parse(&mock_server.uri()).unwrap(),
            sender,
            authorization_token,
        );

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(result);
    }
}
