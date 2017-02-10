use std;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum TipupError {
    Tipup(String),
}

impl Display for TipupError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            TipupError::Tipup(ref err) => write!(f, "TipupError: {}", err),
        }
    }
}

impl<'a> From<&'a str> for TipupError {
    fn from(err: &'a str) -> TipupError {
        TipupError::Tipup(String::from(err))
    }
}
