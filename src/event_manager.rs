use bson::{self, Bson};
use mongodb::coll::options::{CursorType, FindOptions};
use mongodb::db::{Database, ThreadedDatabase};
use time;

use error::TipupError;
use flag_manager::Flag;

pub struct EventManager {
    duration_seconds: i64,
}

impl EventManager {
    pub fn new(duration_seconds: i64) -> EventManager {
        EventManager {
            duration_seconds: duration_seconds,
        }
    }

    pub fn execute(&self, tipup_db: &Database) -> Result<(), TipupError> {
        //TODO retrieve active events
        
        //get all flags from last 'duration seconds'
        let timestamp = time::now_utc().to_timespec().sec - self.duration_seconds;
        let timestamp_gte = doc!("$gte" => timestamp);
        let proddle_search_document = Some(doc!("timestamp" => timestamp_gte));

        let find_options = Some(FindOptions {
            allow_partial_results: false,
            no_cursor_timeout: false,
            oplog_replay: false,
            skip: None,
            limit: None,
            cursor_type: CursorType::NonTailable,
            batch_size: None,
            comment: None,
            max_time_ms: None,
            modifiers: None,
            projection: None,
            sort: Some(doc!("timestamp" => 1)),
            read_preference: None,
        });

        //iterate over flag documents
        let mut flags: Vec<Flag> = Vec::new();
        let cursor = try!(tipup_db.collection("flags").find(proddle_search_document, find_options));
        for document in cursor {
            let document = try!(document);

            //parse document into Flag
            match bson::from_bson(Bson::Document(document)) {
                Ok(flag) => flags.push(flag),
                Err(_) => return Err(TipupError::from("failed to parse bson document into flag")),
            }
        }

        debug!("processing {} flags", flags.len());

        //TODO compute dbscan algorithm

        //TODO compare current events with old events (change active flag if necessary)
        //TODO update and/or write new events

        unimplemented!();
    }
}
