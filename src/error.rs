use std::error::Error;
use std::fmt;

use rusoto_core::RusotoError;
use rusoto_logs::{CreateLogStreamError, PutLogEventsError};

#[derive(Debug)]
pub enum RusotoLogsError {
    RusotoError(RusotoError<std::convert::Infallible>),
    PutLogEventsError(PutLogEventsError),
    CreateLogStreamError(CreateLogStreamError),
}

impl From<RusotoError<PutLogEventsError>> for RusotoLogsError {
    fn from(error: RusotoError<PutLogEventsError>) -> Self {
        match error {
            RusotoError::Service(error) => Self::PutLogEventsError(error),
            RusotoError::HttpDispatch(err) => Self::RusotoError(RusotoError::HttpDispatch(err)),
            RusotoError::Credentials(err) => Self::RusotoError(RusotoError::Credentials(err)),
            RusotoError::Validation(msg) => Self::RusotoError(RusotoError::Validation(msg)),
            RusotoError::ParseError(msg) => Self::RusotoError(RusotoError::ParseError(msg)),
            RusotoError::Unknown(resp) => Self::RusotoError(RusotoError::Unknown(resp)),
            RusotoError::Blocking => Self::RusotoError(RusotoError::Blocking),
        }
    }
}

impl From<RusotoError<CreateLogStreamError>> for RusotoLogsError {
    fn from(error: RusotoError<CreateLogStreamError>) -> Self {
        match error {
            RusotoError::Service(error) => Self::CreateLogStreamError(error),
            RusotoError::HttpDispatch(err) => Self::RusotoError(RusotoError::HttpDispatch(err)),
            RusotoError::Credentials(err) => Self::RusotoError(RusotoError::Credentials(err)),
            RusotoError::Validation(msg) => Self::RusotoError(RusotoError::Validation(msg)),
            RusotoError::ParseError(msg) => Self::RusotoError(RusotoError::ParseError(msg)),
            RusotoError::Unknown(resp) => Self::RusotoError(RusotoError::Unknown(resp)),
            RusotoError::Blocking => Self::RusotoError(RusotoError::Blocking),
        }
    }
}

impl fmt::Display for RusotoLogsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RusotoLogsError::RusotoError(err) => write!(f, "{}", err),
            RusotoLogsError::PutLogEventsError(err) => write!(f, "{}", err),
            RusotoLogsError::CreateLogStreamError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for RusotoLogsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RusotoLogsError::RusotoError(ref err) => Error::source(err),
            RusotoLogsError::PutLogEventsError(ref err) => Error::source(err),
            RusotoLogsError::CreateLogStreamError(ref err) => Error::source(err),
        }
    }
}
