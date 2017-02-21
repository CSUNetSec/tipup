use bson::{Bson, Document};
use bson::ordered::OrderedDocument;
use mongodb::db::{Database, ThreadedDatabase};
use rustc_serialize::json::{Json, ToJson};

use error::TipupError;

use std::collections::BTreeMap;

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

#[derive(Debug)]
pub enum FlagStatus {
    Unreachable,
    Warning,
    Internal,
}

impl ToJson for Flag {
    fn to_json(&self) -> Json {
        let mut map = BTreeMap::new();
        map.insert(String::from("timestamp"), self.timestamp.to_json());
        map.insert(String::from("hostname"), self.hostname.to_json());
        map.insert(String::from("ip_address"), self.ip_address.to_json());
        map.insert(String::from("domain"), self.domain.to_json());
        if let Some(ref domain_ip_address) = self.domain_ip_address {
            map.insert(String::from("domain_ip_address"), domain_ip_address.to_json());
        }
        map.insert(String::from("url"), self.url.to_json());
        let _ = match self.status {
            FlagStatus::Unreachable => map.insert(String::from("status"), "unreachable".to_json()),
            FlagStatus::Warning => map.insert(String::from("status"), "warning".to_json()),
            FlagStatus::Internal => map.insert(String::from("status"), "internal".to_json()),
        };
        map.insert(String::from("analyzer"), self.analyzer.to_json());
        Json::Object(map)
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
        let json = flag.to_json();
        let document: Document = match Bson::from_json(&json) {
            Bson::Document(document) => document,
            _ => return Err(TipupError::from("failed to parse flag json as Bson::Document")),
        };

        try!(self.tipup_db.collection("flags").insert_one(document, None));

        Ok(())
    }
}

