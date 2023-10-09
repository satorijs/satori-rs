use crate::{CallApiError, Event, Login};

use hyper::StatusCode;
use serde::{Deserialize, Serialize};

mod sdk;
pub use sdk::*;
mod app;
pub use app::*;

#[derive(Serialize, Deserialize, Debug)]
struct Signal<T> {
    op: u8,
    body: T,
}

impl Signal<Event> {
    fn event(event: Event) -> Self {
        Self { op: 0, body: event }
    }
}

impl Signal<()> {
    fn ping() -> Self {
        Self { op: 1, body: () }
    }
    fn pong() -> Self {
        Self { op: 2, body: () }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Identify {
    pub token: String,
    pub sequence: i64,
}

impl Signal<Identify> {
    fn identfy(token: &str, seq: i64) -> Self {
        Self {
            op: 3,
            body: Identify {
                token: token.to_string(),
                sequence: seq,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Logins {
    pub logins: Vec<Login>,
}

impl Signal<Logins> {
    fn ready(logins: Vec<Login>) -> Self {
        Self {
            op: 4,
            body: Logins { logins },
        }
    }
}

impl ToString for Signal<()> {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl ToString for Signal<Identify> {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl ToString for Signal<Logins> {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl CallApiError {
    pub fn into_resp(self) -> (StatusCode, String) {
        match self {
            Self::BadRequest => (StatusCode::BAD_REQUEST, "".to_owned()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "".to_owned()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "".to_owned()),
            Self::NotFound => (StatusCode::NOT_FOUND, "".to_owned()),
            Self::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "".to_owned()),
            Self::ServerError(code) => (StatusCode::from_u16(code).unwrap(), "".to_owned()),
            Self::DeserializeFailed => (StatusCode::BAD_REQUEST, "".to_owned()),
        }
    }
}
