use bson::Bson;
use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::{Flag, FlagStatus};

use std::sync::mpsc::Sender;

pub struct ErrorAnalyzer {
    name: String,
    tx: Sender<Flag>,
}

impl ErrorAnalyzer {
    pub fn new(name: &str, tx: Sender<Flag>) -> Result<ErrorAnalyzer, TipupError> {
        Ok(
            ErrorAnalyzer {
                name: name.to_owned(),
                tx: tx,
            }
        )
    }
}

impl Analyzer for ErrorAnalyzer {
    fn process_result(&mut self, document: &OrderedDocument) -> Result<(), TipupError> {
        //check for internal error
        if let Some(&Bson::Boolean(true)) = document.get("error") {
            let flag = try!(Flag::new(document, FlagStatus::Internal, &self.name));
            try!(self.tx.send(flag));
        }

        //check for measurement error
        if let Some(&Bson::Document(ref result_document)) = document.get("result") {
            if let Some(&Bson::Boolean(true)) = result_document.get("error") {
                //check if there we're more attempts
                let remaining_attempts = match document.get("remaining_attempts") {
                    Some(&Bson::I32(remaining_attempts)) => remaining_attempts,
                    _ => -1,
                };

                if remaining_attempts == 0 {
                    let flag = try!(Flag::new(document, FlagStatus::Unreachable, &self.name));
                    try!(self.tx.send(flag)); 
                }
            }
        }

        Ok(())
    }
}
