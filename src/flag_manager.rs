use bson;
use bson::{Bson, Document};
use bson::ordered::OrderedDocument;
use mongodb::db::{Database, ThreadedDatabase};
use serde::ser::{Serialize, Serializer, SerializeStruct};

use error::TipupError;

#[derive(Debug)]
pub enum FlagStatus {
    Unreachable,
    Warning,
    Internal,
}

#[derive(Debug)]
pub struct Flag {
    timestamp: i64,
    hostname: String,
    ip_address: String,
    domain: String,
    domain_ip_address: Option<String>,
    url: String,
    status: FlagStatus,
    analyzer: String, //name of analyzer
}

impl Serialize for Flag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S:Serializer {
        let mut struc = serializer.serialize_struct("flag", 8)?;
        struc.serialize_field("timestamp", &self.timestamp)?;
        struc.serialize_field("hostname", &self.hostname)?;
        struc.serialize_field("ip_address", &self.ip_address)?;
        struc.serialize_field("domain", &self.domain)?;
        struc.serialize_field("domain_ip", &self.domain_ip_address)?;
        struc.serialize_field("url", &self.url)?;
        match self.status {
            FlagStatus::Unreachable => struc.serialize_field("status", "unreachable")?,
            FlagStatus::Warning => struc.serialize_field("status", "warning")?,
            FlagStatus::Internal => struc.serialize_field("status", "internal")?,
        }
        struc.serialize_field("analyzer", &self.analyzer)?;
        struc.end()
    }
}

impl Flag {
    pub fn new(document: &OrderedDocument, status: FlagStatus, analyzer: &str) -> Result<Flag, TipupError> {
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
                status: status,
                analyzer: analyzer.to_owned(),
            }
        )
    }
}

fn parse_string(document: &OrderedDocument, name: &str) -> Result<String, TipupError> {
    match document.get(name) {
        Some(&Bson::String(ref value)) => Ok(value.to_owned()),
        _ => Err(TipupError::from("failed to parse hostname as string")),
    }
}

pub struct FlagManager<'a> {
    tipup_db: &'a Database,
}

impl<'a> FlagManager<'a> {
    pub fn new(tipup_db: &'a Database) -> FlagManager {
        FlagManager {
            tipup_db: tipup_db,
        }
    }

    pub fn process_flag(&mut self, flag: &Flag) -> Result<(), TipupError> {
        //write to database
        let document: Document = match bson::to_bson(flag) {
            Ok(Bson::Document(document)) => document,
            _ => return Err(TipupError::from("failed to parse flag json as Bson::Document")),
        };

        try!(self.tipup_db.collection("flags").insert_one(document, None));

        Ok(())
    }
}
