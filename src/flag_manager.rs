use bson::Bson;
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

pub fn create_flag(document: &OrderedDocument, level: u8, internal_error: bool, analyzer: &str) -> Result<Flag, TipupError> {
    let timestamp = match document.get("timestamp") {
        Some(&Bson::I64(timestamp)) => timestamp,
        _ => return Err(TipupError::from("failed to parse timestamp as i64")),
    };

    let domain_ip_address = match document.get("result") {
        Some(&Bson::Document(ref result_document)) => {
            match result_document.get("domain_ip") {
                Some(&Bson::String(ref domain_ip)) => Some(domain_ip.to_owned()),
                _ => None,
            }
        },
        _ => None,
    };

    Ok(
        Flag {
            timestamp: timestamp,
            hostname: try!(parse_string(document, "hostname")),
            ip_address: try!(parse_string(document, "ip_address")),
            domain: try!(parse_string(document, "domain")),
            domain_ip_address: domain_ip_address,
            url: try!(parse_string(document, "url")),
            level: level,
            internal_error: internal_error,
            analyzer: analyzer.to_owned(),
        }
    )
}

fn parse_string(document: &OrderedDocument, name: &str) -> Result<String, TipupError> {
    match document.get(name) {
        Some(&Bson::String(ref value)) => Ok(value.to_owned()),
        _ => Err(TipupError::from("failed to parse hostname as string")),
    }
}
