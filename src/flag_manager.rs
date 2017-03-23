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
    pub status: FlagStatus,
    pub analyzer: String,
}

impl Flag {
    pub fn new(document: &OrderedDocument, status: FlagStatus, analyzer: &str) -> Result<Flag, TipupError> {
        let measurement_id = match document.get("_id") {
            Some(&Bson::ObjectId(ref measurement_id)) => measurement_id.clone(),
            _ => return Err(TipupError::from("failed to parse measurement '_id' as ObjectId")),
        };

        Ok(
            Flag {
                id: ObjectId::new().unwrap(),
                measurement_id: measurement_id,
                status: status,
                analyzer: analyzer.to_owned(),
            }
        )
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
