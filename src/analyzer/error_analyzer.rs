use bson::Bson;
use bson::ordered::OrderedDocument;
use chan::Sender;

use analyzer::Analyzer;
use error::TipupError;
use flag_manager::Flag;

pub struct ErrorAnalyzer {
    name: String,
    status: String,
    fields: Vec<String>,
    flag_tx: Sender<Flag>,
}

impl ErrorAnalyzer {
    pub fn new(name: &str, status: &str, fields: Vec<String>, flag_tx: Sender<Flag>) -> Result<ErrorAnalyzer, TipupError> {
        Ok(
            ErrorAnalyzer {
                name: name.to_owned(),
                status: status.to_owned(),
                fields: fields,
                flag_tx: flag_tx,
            }
        )
    }
}

impl Analyzer for ErrorAnalyzer {
    fn process_measurement(&mut self, document: &OrderedDocument) -> Result<(), TipupError> {
        //check if fields exist
        for field in self.fields.iter() {
            if document.contains_key(field) {
                let flag = try!(Flag::new(document, &self.status, &self.name));
                self.flag_tx.send(flag);
                break;
            }
        }

        Ok(())
    }
}
