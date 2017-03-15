use bson::{self, Bson, Document};
use bson::oid::ObjectId;
use bson::ordered::OrderedDocument;
use mongodb::db::{Database, ThreadedDatabase};

use error::TipupError;

#[derive(Debug, Deserialize, Serialize)]
pub enum FlagStatus {
    Unreachable,
    Warning,
    Internal,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Flag {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub measurement_id: ObjectId,
    pub timestamp: i64,
    pub hostname: String,
    pub ip_address: String,
    pub domain: String,
    pub domain_ip_address: Option<String>,
    pub url: String,
    pub status: FlagStatus,
    pub analyzer: String,
}

impl Flag {
    pub fn new(document: &OrderedDocument, status: FlagStatus, analyzer: &str) -> Result<Flag, TipupError> {
        let measurement_id = match document.get("_id") {
            Some(&Bson::ObjectId(ref measurement_id)) => measurement_id.clone(),
            _ => return Err(TipupError::from("failed to parse measurement _id as ObjectId")),
        };

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
                id: ObjectId::new().unwrap(),
                measurement_id: measurement_id,
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

pub struct FlagManager {
}

impl FlagManager {
    pub fn new() -> FlagManager {
        FlagManager {
        }
    }

    pub fn process_flag(&mut self, flag: &Flag, tipup_db: &Database) -> Result<(), TipupError> {
        //write to database
        let document: Document = match bson::to_bson(flag) {
            Ok(Bson::Document(document)) => document,
            _ => return Err(TipupError::from("failed to parse flag json as Bson::Document")),
        };

        try!(tipup_db.collection("flags").insert_one(document, None));

        Ok(())
    }
}
