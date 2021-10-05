use quick_xml::Error as XMLError;
use std::{str::Utf8Error, string::FromUtf8Error};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    CannotDecode,
    MalformedXML(String),
    ContainerCannotMove,
    NotFound,
    HasAParent,
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
            err => Error::MalformedXML(format!("{:?}", err)),
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
