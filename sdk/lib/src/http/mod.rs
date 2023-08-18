//! The HTTP module which provides functions for performing requests to
//! endpoints in Discord's API.
//!
//! An important function of the REST API is ratelimiting. Requests to endpoints
//! are ratelimited to prevent spam, and once ratelimited Discord will stop
//! performing requests. The library implements protection to pre-emptively
//! ratelimit, to ensure that no wasted requests are made.
//!
//! The HTTP module comprises of two types of requests:
//!
//! - REST API requests, which require an authorization token;
//! - Other requests, which do not require an authorization token.
//!
//! The former require a [`Client`] to have logged in, while the latter may be
//! made regardless of any other usage of the library.
//!
//! If a request spuriously fails, it will be retried once.
//!
//! Note that you may want to perform requests through a [model]s'
//! instance methods where possible, as they each offer different
//! levels of a high-level interface to the HTTP module.
//!
//! [`Client`]: crate::Client
//! [model]: crate::model

pub mod client;
pub mod error;
pub mod multipart;
pub mod request;
pub mod routing;
pub mod typing;
mod utils;

use http_req::request::Method;

pub use self::client::*;
pub use self::error::Error as HttpError;
pub use self::typing::*;
use crate::model::*;

/// This trait will be required by functions that need [`Http`] and can
/// optionally use a [`Cache`] to potentially avoid REST-requests.
///
/// The types [`Context`] and [`Http`] implement this trait
/// and thus passing these to functions expecting `impl CacheHttp` is possible. For the full list
/// of implementations, see the Implementors and Implementations on Foreign Types section in the
/// generated docs.
///
/// In a situation where you have the `cache`-feature enabled but you do not
/// pass a cache, the function will behave as if no `cache`-feature is active.
///
/// If you are calling a function that expects `impl CacheHttp` as argument
/// and you wish to utilise the `cache`-feature but you got no access to a
/// [`Context`], you can pass a tuple of `(&Arc<Cache>, &Http)`.

/// An method used for ratelimiting special routes.
///
/// This is needed because [`reqwest`]'s [`Method`] enum does not derive Copy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LightMethod {
    /// Indicates that a route is for the `DELETE` method only.
    Delete,
    /// Indicates that a route is for the `GET` method only.
    Get,
    /// Indicates that a route is for the `PATCH` method only.
    Patch,
    /// Indicates that a route is for the `POST` method only.
    Post,
    /// Indicates that a route is for the `PUT` method only.
    Put,
}

impl LightMethod {
    #[must_use]
    pub fn reqwest_method(self) -> Method {
        match self {
            Self::Delete => Method::DELETE,
            Self::Get => Method::GET,
            Self::Patch => Method::PATCH,
            Self::Post => Method::POST,
            Self::Put => Method::PUT,
        }
    }
}

/// Representation of the method of a query to send for the [`get_guilds`]
/// function.
///
/// [`get_guilds`]: Http::get_guilds
#[non_exhaustive]
pub enum GuildPagination {
    /// The Id to get the guilds after.
    After(GuildId),
    /// The Id to get the guilds before.
    Before(GuildId),
}

/// Representation of the method of a query to send for the [`get_scheduled_event_users`] function.
///
/// [`get_scheduled_event_users`]: Http::get_scheduled_event_users
#[non_exhaustive]
pub enum UserPagination {
    /// The Id to get the users after.
    After(UserId),
    /// The Id to get the users before.
    Before(UserId),
}
