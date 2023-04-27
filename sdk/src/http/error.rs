use std::error::Error as StdError;
use std::fmt;

use http_req::error::Error as ReqwestError;
use http_req::response::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::{ParseError as UrlError, Url};

use crate::http::utils::deserialize_errors;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub struct DiscordJsonError {
    /// The error code.
    pub code: isize,
    /// The error message.
    pub message: String,
    /// The full explained errors with their path in the request
    /// body.
    #[serde(default, deserialize_with = "deserialize_errors")]
    pub errors: Vec<DiscordJsonSingleError>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DiscordJsonSingleError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
    /// The path to the error in the request body itself, dot separated.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorResponse {
    pub status_code: StatusCode,
    pub url: Url,
    pub error: DiscordJsonError,
}

impl ErrorResponse {
    // We need a freestanding from-function since we cannot implement an async
    // From-trait.
    pub async fn from_response(r: Response) -> Self {
        ErrorResponse {
            status_code: r.status_code(),
            url: "https://nourl.icu/".try_into().unwrap(),
            error:DiscordJsonError {
                code: -1,
                message: format!("[Serenity] Could not decode json when receiving error response from discord:, {}", r.reason()),
                errors: vec![],
            },
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// When a non-successful status code was received for a request.
    UnsuccessfulRequest(ErrorResponse),
    /// When the decoding of a ratelimit header could not be properly decoded
    /// into an `i64` or `f64`.
    RateLimitI64F64,
    /// When the decoding of a ratelimit header could not be properly decoded
    /// from UTF-8.
    RateLimitUtf8,
    /// When parsing an URL failed due to invalid input.
    Url(UrlError),
    /// When parsing a Webhook fails due to invalid input.
    InvalidWebhook,
    /// Reqwest's Error contain information on why sending a request failed.
    Request(ReqwestError),
    /// When using a proxy with an invalid scheme.
    InvalidScheme,
    /// When using a proxy with an invalid port.
    InvalidPort,
    /// When an application id was expected but missing.
    ApplicationIdMissing,
}

impl Error {
    // We need a freestanding from-function since we cannot implement an async
    // From-trait.
    pub async fn from_response(r: Response) -> Self {
        ErrorResponse::from_response(r).await.into()
    }

    /// Returns true when the error is caused by an unsuccessful request
    #[must_use]
    pub fn is_unsuccessful_request(&self) -> bool {
        matches!(self, Self::UnsuccessfulRequest(_))
    }

    /// Returns true when the error is caused by the url containing invalid input
    #[must_use]
    pub fn is_url_error(&self) -> bool {
        matches!(self, Self::Url(_))
    }

    /// Returns the status code if the error is an unsuccessful request
    #[must_use]
    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            Self::UnsuccessfulRequest(res) => Some(res.status_code),
            _ => None,
        }
    }
}

impl From<ErrorResponse> for Error {
    fn from(error: ErrorResponse) -> Error {
        Error::UnsuccessfulRequest(error)
    }
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Error {
        Error::Request(error)
    }
}

impl From<UrlError> for Error {
    fn from(error: UrlError) -> Error {
        Error::Url(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsuccessfulRequest(e) => {
                f.write_str(&e.error.message)?;

                // Put Discord's human readable error explanations in parentheses
                let mut errors_iter = e.error.errors.iter();
                if let Some(error) = errors_iter.next() {
                    f.write_str(" (")?;
                    f.write_str(&error.path)?;
                    f.write_str(": ")?;
                    f.write_str(&error.message)?;
                    for error in errors_iter {
                        f.write_str(", ")?;
                        f.write_str(&error.path)?;
                        f.write_str(": ")?;
                        f.write_str(&error.message)?;
                    }
                    f.write_str(")")?;
                }

                Ok(())
            }
            Self::RateLimitI64F64 => f.write_str("Error decoding a header into an i64 or f64"),
            Self::RateLimitUtf8 => f.write_str("Error decoding a header from UTF-8"),
            Self::Url(_) => f.write_str("Provided URL is incorrect."),
            Self::InvalidWebhook => f.write_str("Provided URL is not a valid webhook."),
            Self::Request(_) => f.write_str("Error while sending HTTP request."),
            Self::InvalidScheme => f.write_str("Invalid Url scheme."),
            Self::InvalidPort => f.write_str("Invalid port."),
            Self::ApplicationIdMissing => f.write_str("Application id was expected but missing."),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Url(inner) => Some(inner),
            Self::Request(inner) => Some(inner),
            _ => None,
        }
    }
}
