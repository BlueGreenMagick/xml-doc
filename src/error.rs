use crate::ElementId;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    RootCannotMove,
    ElementNotExist(ElementId),
    LazyError(quick_xml::Error),
}

impl From<quick_xml::Error> for Error {
    fn from(err: quick_xml::Error) -> Error {
        Error::LazyError(err)
    }
}
