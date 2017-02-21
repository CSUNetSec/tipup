use bson::ordered::OrderedDocument;

use error::TipupError;

#[derive(Debug)]
pub struct Flag {
    timestamp: i64,
    hostname: String,
    ip_address: String,
    domain: String,
    domain_ip_address: Option<String>,
    url: String,
    level: u8, //1-10 value for flag severity
    internal_error: bool,
    analyzer: String, //name of analyzer
}

pub struct FlagManager {
}

impl FlagManager {
    pub fn new() -> FlagManager {
        FlagManager {
        }
    }

    pub fn process_flag(&mut self, flag: &Flag) -> Result<(), TipupError> {
        println!("TODO PROCESS FLAG: {:?}", flag);

        Ok(())
    }
}

pub fn create_flag(document: &OrderedDocument, level: u8, internal_error: bool) -> Result<Flag, TipupError> {
    unimplemented!();
}
