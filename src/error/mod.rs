use std::backtrace::Backtrace;

use reqwest::StatusCode;
use rocket::{http::Status, response::Responder, Response};

#[derive(Debug, Clone)]
pub struct Error {
    pub status: Status,
    pub msg: Option<String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.status)?;
        match self.msg.clone() {
            Some(msg) => write!(f, ": {}", msg),
            None => write!(f, ": No details"),
        }
    }
}

impl From<rocket_db_pools::mongodb::error::Error> for Error {
    fn from(val: rocket_db_pools::mongodb::error::Error) -> Self {
        Error {
            status: Status::InternalServerError,
            msg: Some(val.to_string()),
        }
    }
}

impl From<Status> for Error {
    fn from(val: Status) -> Self {
        Error {
            status: val.clone(),
            msg: Some(val.to_string()),
        }
    }
}

impl From<StatusCode> for Error {
    fn from(val: StatusCode) -> Self {
        Error {
            status: Status::new(val.as_u16()),
            msg: Some(val.to_string()),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(val: reqwest::Error) -> Self {
        Error {
            status: Status::new(
                val.status()
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                    .as_u16(),
            ),
            msg: Some(val.to_string()),
        }
    }
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        error!("ERROR: {}", self);
        Response::build_from(self.status.respond_to(request)?).ok()
    }
}
