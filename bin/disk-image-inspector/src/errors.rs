use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub(crate) enum ImageError {
    InvalidSignature,
}

impl Display for ImageError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature"),
        }
    }
}

impl Error for ImageError {}
