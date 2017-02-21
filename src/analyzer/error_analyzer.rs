use bson::Bson;
use bson::ordered::OrderedDocument;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::Flag;

use std::sync::mpsc::Sender;

pub struct ErrorAnalyzer {
    name: String,
    tx: Sender<Flag>,
}

impl ErrorAnalyzer {
    pub fn new(name: &str, _: &Vec<Bson>, tx: Sender<Flag>) -> Result<ErrorAnalyzer, TipupError> {
        Ok(
            ErrorAnalyzer {
                name: name.to_owned(),
                tx: tx,
            }
        )
    }
}

impl Analyzer for ErrorAnalyzer {
    fn process_result(&self, document: &OrderedDocument) -> Result<(), TipupError> {
        //check for internal error
        if let Some(&Bson::Boolean(true)) = document.get("error") {
            let flag = try!(Flag::new(document, 10, true, &self.name));
            try!(self.tx.send(flag));
        }

        //check for measurement error
        if let Some(&Bson::Document(ref result_document)) = document.get("result") {
            if let Some(&Bson::Boolean(true)) = result_document.get("error") {
                let flag = try!(Flag::new(document, 8, false, &self.name));
                try!(self.tx.send(flag));
            }
        }

        Ok(())
    }
}
