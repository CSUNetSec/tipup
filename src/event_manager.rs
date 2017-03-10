use bson::{self, Bson};
use dbscan::{DBSCAN, SymmetricMatrix};
use mongodb::coll::options::{CursorType, FindOptions};
use mongodb::db::{Database, ThreadedDatabase};
use time;

use error::TipupError;
use flag_manager::Flag;

use std::collections::HashMap;

pub struct EventManager {
    duration_seconds: i64,
    maximum_distance: f64,
    minimum_points: usize,
}

impl EventManager {
    pub fn new(duration_seconds: i64) -> EventManager {
        EventManager {
            duration_seconds: duration_seconds,
            maximum_distance: 1.5,
            minimum_points: 4,
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

        //compute dbscan algorithm
        let mut dbscan = DBSCAN::new(self.maximum_distance, self.minimum_points);
        let mut symmetric_matrix = SymmetricMatrix::<f64>::new(flags.len());
        for i in 0..flags.len()-1 {
            for j in i+1..flags.len() {
                symmetric_matrix.set(i, j, compute_flag_distance(&flags[i], &flags[j]));
            }
        }

        let clusters = dbscan.perform_clustering(&symmetric_matrix);

        //create hashmap of clusters
        let mut cluster_map: HashMap<usize, Vec<&Flag>> = HashMap::new();
        let mut unclustered = Vec::new();
        for (i, cluster) in clusters.iter().enumerate() {
            let ref flag = flags[i];
            match cluster {
                &Some(cluster_id) => cluster_map.entry(cluster_id).or_insert(Vec::new()).push(flag),
                &None => unclustered.push(flag),
            }
        }

        //TODO compare current events with old events (change active flag if necessary)
        //TODO update and/or write new events

        unimplemented!();
    }
}

fn compute_flag_distance(flag_one: &Flag, flag_two: &Flag) -> f64 {
    //timestamp
    let timestamp_difference = (flag_one.timestamp - flag_two.timestamp).abs();
    let timestamp_score = match timestamp_difference {
        0 ... 86400 => timestamp_difference as f64 / 86400.0,
        _ => 1.0,
    };

    //TODO status
    let status_score = 0.0;

    //domains
    let domain_score = match flag_one.domain.eq(&flag_two.domain) {
        true => 0.0,
        false => 1.0,
    };

    //urls - TODO fuzzy match
    let url_score = match flag_one.url.eq(&flag_two.url) {
        true => 0.0,
        false => 1.0,
    };

    return (timestamp_score) + (status_score) + (domain_score * 1.3) + (url_score);
}
