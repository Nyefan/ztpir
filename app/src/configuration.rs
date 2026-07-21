use crate::domain::SubscriberEmail;
use secrecy::{ExposeSecret, SecretString};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub interface: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DatabaseSettings {
    pub require_ssl: bool,
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub schema_name: String,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .ssl_mode(if self.require_ssl {
                PgSslMode::Require
            } else {
                PgSslMode::Prefer
            })
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .database(&self.schema_name)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
    pub timeout_milliseconds: u16,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }
}

// TODO: good god - your code should never, NEVER, know what environment it's running in - behavior
//       MUST be controlled exclusively by configuration that is environment agnostic.  Anything
//       else is malpractice.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            _ => Err(format!("Invalid environment: {}", s)),
        }
    }
}

// TODO: oncelock
// TODO: this is all really bad - we need to use a defined subset of the spring properties
//       hierarchy and stick to that.  We also need to prefix all environment variables with
//       ZTPIR_ (or an overrideable prefix).  Neither clap nor config supports this, and it's a
//       glaring missing element of the rust ecosystem.
pub fn get_config() -> Result<Settings, config::ConfigError> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let config_directory = cwd.join("config");
    let environment: Environment = std::env::var("ZTPIR_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse environment");
    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(config_directory.join("default.yaml")))
        .add_source(config::File::from(
            config_directory.join(&environment_filename),
        ))
        .add_source(
            config::Environment::with_prefix("ZTPIR")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;
    settings.try_deserialize::<Settings>()
}
