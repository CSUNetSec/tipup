extern crate clap;

use std;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum TipupError {
    Clap(clap::Error),
    Tipup(String),
}

impl Display for TipupError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            TipupError::Clap(ref err) => write!(f, "ClapError: {}", err),
            TipupError::Tipup(ref err) => write!(f, "TipupError: {}", err),
        }
    }
}

impl From<clap::Error> for TipupError {
    fn from(err: clap::Error) -> TipupError {
        TipupError::Clap(err)
    }
}

impl<'a> From<&'a str> for TipupError {
    fn from(err: &'a str) -> TipupError {
        TipupError::Tipup(String::from(err))
    }
}
