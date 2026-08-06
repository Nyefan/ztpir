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
