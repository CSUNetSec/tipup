extern crate clap;
extern crate mongodb;

use std;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum TipupError {
    Clap(clap::Error),
    MongoDB(mongodb::Error),
    Tipup(String),
}

impl Display for TipupError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            TipupError::Clap(ref err) => write!(f, "ClapError: {}", err),
            TipupError::MongoDB(ref err) => write!(f, "MongoDBError: {}", err),
            TipupError::Tipup(ref err) => write!(f, "TipupError: {}", err),
        }
    }
}

impl From<clap::Error> for TipupError {
    fn from(err: clap::Error) -> TipupError {
        TipupError::Clap(err)
    }
}

impl From<mongodb::Error> for TipupError {
    fn from(err: mongodb::Error) -> TipupError {
        TipupError::MongoDB(err)
    }
}

impl<'a> From<&'a str> for TipupError {
    fn from(err: &'a str) -> TipupError {
        TipupError::Tipup(String::from(err))
    }
}
