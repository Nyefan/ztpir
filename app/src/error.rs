use crate::error::Error::Unhandleable;
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use anyhow::Context;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Handleable(anyhow::Error),
    Unhandleable(anyhow::Error, StatusCode),
    Unknown(anyhow::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Handleable(e) => Display::fmt(e, f),
            Error::Unhandleable(e, _) => Display::fmt(e, f),
            Error::Unknown(e) => Display::fmt(e, f),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Handleable(_) => StatusCode::INTERNAL_SERVER_ERROR, // Handleable errors shouldn't be returned to http client, and doing so is an error
            Error::Unhandleable(_, code) => *code,
            Error::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub trait OnErrorReturn<T> {
    fn on_error_return(self, status_code: StatusCode, body: &'static str) -> Result<T, Error>;
}

impl<T, E> OnErrorReturn<T> for Result<T, E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn on_error_return(self, status_code: StatusCode, body: &'static str) -> Result<T, Error> {
        self.context(body)
            .map_err(|e| Error::Unhandleable(e, status_code))
    }
}

pub trait OnErrorStatus<T> {
    fn on_error_status(self, status_code: StatusCode) -> Result<T, Error>;
}

impl<T> OnErrorStatus<T> for Result<T, anyhow::Error> {
    fn on_error_status(self, status_code: StatusCode) -> Result<T, Error> {
        self.map_err(|e| Error::Unhandleable(e, status_code))
    }
}

pub trait WhenNoneReturn<T> {
    fn when_none_return(self, status_code: StatusCode, body: &'static str) -> Result<T, Error>;
}

impl<T> WhenNoneReturn<T> for Option<T> {
    fn when_none_return(self, status_code: StatusCode, body: &'static str) -> Result<T, Error> {
        self.context(body).map_err(|e| Unhandleable(e, status_code))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::anyhow;
    use fake::Fake;
    use fake::faker::lorem::en::Sentence;

    #[tokio::test]
    async fn handleable_error_returns_500_if_not_handled() {
        let error = Error::Handleable(anyhow!(Sentence(1..2).fake::<String>()));
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn unhandleable_error_returns_input_status_code() {
        let code = StatusCode::from_u16((100..1000).fake()).unwrap();
        let error = Error::Unhandleable(anyhow!(Sentence(1..2).fake::<String>()), code);
        assert_eq!(error.status_code(), code);
    }

    #[tokio::test]
    async fn unknown_error_returns_500() {
        let error = Error::Unknown(anyhow!(Sentence(1..2).fake::<String>()));
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn handleable_error_displays_as_input_str() {
        let body = Sentence(1..2).fake::<String>();
        let error = Error::Handleable(anyhow!(body.clone()));
        let display = format!("{error}");
        assert_eq!(display, body)
    }

    #[tokio::test]
    async fn unhandleable_error_displays_as_input_str() {
        let body = Sentence(1..2).fake::<String>();
        let error = Error::Unhandleable(
            anyhow!(body.clone()),
            StatusCode::from_u16((100..1000).fake()).unwrap(),
        );
        let display = format!("{error}");
        assert_eq!(display, body)
    }

    #[tokio::test]
    async fn unknown_error_displays_as_input_str() {
        let body = Sentence(1..2).fake::<String>();
        let error = Error::Unknown(anyhow!(body.clone()));
        let display = format!("{error}");
        assert_eq!(display, body)
    }
}
