use bson::Bson;
use bson::ordered::OrderedDocument;
use chan::Sender;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::{Flag, FlagStatus};

pub struct ErrorAnalyzer {
    name: String,
    flag_tx: Sender<Flag>,
}

impl ErrorAnalyzer {
    pub fn new(name: &str, flag_tx: Sender<Flag>) -> Result<ErrorAnalyzer, TipupError> {
        Ok(
            ErrorAnalyzer {
                name: name.to_owned(),
                flag_tx: flag_tx,
            }
        )
    }
}

impl Analyzer for ErrorAnalyzer {
    fn process_result(&mut self, document: &OrderedDocument) -> Result<(), TipupError> {
        //check for internal error
        if let Some(&Bson::Boolean(true)) = document.get("error") {
            let flag = try!(Flag::new(document, FlagStatus::Internal, &self.name));
            self.flag_tx.send(flag);
        }

        //check for measurement error
        if let Some(&Bson::Document(ref result_document)) = document.get("result") {
            if let Some(&Bson::Boolean(true)) = result_document.get("error") {
                //check if there we're more attempts
                let remaining_attempts = match document.get("remaining_attempts") {
                    Some(&Bson::I64(remaining_attempts)) => remaining_attempts as i32,
                    Some(&Bson::I32(remaining_attempts)) => remaining_attempts,
                    _ => -1,
                };

                if remaining_attempts == 0 {
                    let flag = try!(Flag::new(document, FlagStatus::Unreachable, &self.name));
                    self.flag_tx.send(flag); 
                }
            }
        }

        Ok(())
    }
}
