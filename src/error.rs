use quick_xml::Error as XMLError;
use std::{str::Utf8Error, string::FromUtf8Error};

/// Wrapper around `std::Result`
pub type Result<T> = std::result::Result<T, Error>;

/// Error types
#[derive(Debug)]
pub enum Error {
    /// [`std::io`] related error.
    Io(std::io::Error),
    /// Decoding related error.
    /// Maybe the XML declaration has an encoding value that it doesn't recognize,
    /// or it doesn't match its actual encoding,
    CannotDecode,
    /// Assorted errors while parsing XML.
    MalformedXML(String),
    /// The container element cannot have a parent.
    /// Use `element.is_container()` to check if it is a container before
    /// assigning it to another parent.
    ContainerCannotMove,
    /// You need to call `element.detatch()` before assigning another parent.
    HasAParent,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO Error: {}", err),
            Error::CannotDecode => write!(f, "Cannot decode XML"),
            Error::MalformedXML(err) => write!(f, "Malformed XML: {}", err),
            Error::ContainerCannotMove => write!(f, "Container element cannot move"),
            Error::HasAParent => write!(
                f,
                "Element already has a parent. Call detatch() before changing parent."
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<XMLError> for Error {
    fn from(err: XMLError) -> Error {
        match err {
            XMLError::EndEventMismatch { expected, found } => Error::MalformedXML(format!(
                "Closing tag mismatch. Expected {}, found {}",
                expected, found,
            )),
            XMLError::Io(err) => Error::Io(err),
            XMLError::Utf8(_) => Error::CannotDecode,
            err => Error::MalformedXML(err.to_string()),
        }
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_: FromUtf8Error) -> Error {
        Error::CannotDecode
    }
}
impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Error {
        Error::CannotDecode
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}
