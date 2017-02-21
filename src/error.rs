extern crate clap;
extern crate mongodb;

use flag_manager::Flag;

use std;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum TipupError {
    Clap(clap::Error),
    MongoDB(mongodb::Error),
    Send(std::sync::mpsc::SendError<Flag>),
    Tipup(String),
}

impl Display for TipupError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            TipupError::Clap(ref err) => write!(f, "ClapError: {}", err),
            TipupError::MongoDB(ref err) => write!(f, "MongoDBError: {}", err),
            TipupError::Send(ref err) => write!(f, "Send: {}", err),
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

impl From<std::sync::mpsc::SendError<Flag>> for TipupError {
    fn from(err: std::sync::mpsc::SendError<Flag>) -> TipupError {
        TipupError::Send(err)
    }
}

impl<'a> From<&'a str> for TipupError {
    fn from(err: &'a str) -> TipupError {
        TipupError::Tipup(String::from(err))
    }
}

impl From<String> for TipupError {
    fn from(err: String) -> TipupError {
        TipupError::Tipup(err)
    }
}
