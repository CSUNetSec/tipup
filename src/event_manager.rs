use bson::{self, Bson, Document};
use bson::oid::ObjectId;
use dbscan::{DBSCAN, SymmetricMatrix};
use mongodb::db::{Database, ThreadedDatabase};
use time;

use error::TipupError;
use flag_manager::Flag;

use std;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    #[serde(rename = "_id")]
    id: ObjectId,
    minimum_timestamp: i64,
    maximum_timestamp: i64,
    domain: String,
    urls: HashSet<String>,
    flag_ids: HashSet<ObjectId>,
}

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
        let timestamp = time::now_utc().to_timespec().sec - self.duration_seconds;

        //retrieve active events
        let mut active_events: HashMap<String, Vec<Event>> = HashMap::new();
        let timestamp_gte = doc!("$gte" => timestamp);
        let event_search_document = Some(doc!("maximum_timestamp" => timestamp_gte));
        let cursor = try!(tipup_db.collection("events").find(event_search_document, None));
        for document in cursor {
            let document = try!(document);

            //parse document into Flag
            let event: Event = match bson::from_bson(Bson::Document(document)) {
                Ok(event) => event,
                Err(_) => return Err(TipupError::from("failed to parse bson document into event")),
            };

            active_events.entry(event.domain.clone()).or_insert(Vec::new()).push(event);
        }
 
        //iterate over flag documents
        let mut flags: Vec<Flag> = Vec::new();
        let timestamp_gte = doc!("$gte" => timestamp);
        let flag_search_document = Some(doc!("timestamp" => timestamp_gte));
        let cursor = try!(tipup_db.collection("flags").find(flag_search_document, None));
        for document in cursor {
            let document = try!(document);

            //parse document into Flag
            match bson::from_bson(Bson::Document(document)) {
                Ok(flag) => flags.push(flag),
                Err(_) => return Err(TipupError::from("failed to parse bson document into flag")),
            }
        }

        if flags.len() == 0 {
            return Ok(());
        }

        //execute dbscan algorithm
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

        //process new events
        for flags in cluster_map.values() {
            if let Err(e) = process_event(&flags, &mut active_events, tipup_db) {
                error!("{}", e);
            }
        }

        Ok(())
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

fn process_event(flags: &Vec<&Flag>, active_events: &mut HashMap<String, Vec<Event>>, tipup_db: &Database) -> Result<(), TipupError> {
    let mut minimum_timestamp = i64::max_value();
    let mut maximum_timestamp = i64::min_value();
    let mut domains = HashSet::new();
    let mut urls = HashSet::new();
    let mut flag_ids = HashSet::new();

    for flag in flags {
        minimum_timestamp = std::cmp::min(minimum_timestamp, flag.timestamp);
        maximum_timestamp = std::cmp::max(maximum_timestamp, flag.timestamp);
        domains.insert(flag.domain.clone());
        urls.insert(flag.url.clone());
        flag_ids.insert(flag.id.clone());
    }

    if domains.len() != 1 {
        return Err(TipupError::from(format!("cluster with {} domain(s) found", domains.len())));
    }

    //create event object and check if event already exists
    let event = Event {
        id: ObjectId::new().unwrap(),
        minimum_timestamp: minimum_timestamp,
        maximum_timestamp: maximum_timestamp,
        domain: domains.into_iter().next().unwrap(),
        urls: urls,
        flag_ids: flag_ids,
    };

    //compare current event with active events from same domain (based on timestamps)
    if let Some(active_events) = active_events.get_mut(&event.domain) {
        for active_event in active_events.iter_mut() {
            if (event.minimum_timestamp >= active_event.minimum_timestamp && event.minimum_timestamp <= active_event.maximum_timestamp)
                    || (event.maximum_timestamp <= active_event.maximum_timestamp && event.maximum_timestamp >= active_event.minimum_timestamp) {
                //update active event in database
                active_event.minimum_timestamp = std::cmp::min(event.minimum_timestamp, active_event.minimum_timestamp);
                active_event.maximum_timestamp = std::cmp::min(event.maximum_timestamp, active_event.maximum_timestamp);

                for url in event.urls.iter() {
                    active_event.urls.insert(url.clone());
                }

                let mut update = false;
                for flag_id in event.flag_ids.iter() {
                    update = active_event.flag_ids.insert(flag_id.clone()) || update;
                }

                //update document in mongodb
                if update {
                    let object_id = active_event.id.clone();
                    let search_document = doc!("_id" => object_id);
                    let event_document: Document = match bson::to_bson(active_event) {
                        Ok(Bson::Document(event_document)) => event_document,
                        _ => return Err(TipupError::from("failed to parse updated event document as Bson::Document")),
                    };
                    try!(tipup_db.collection("events").find_one_and_replace(search_document, event_document, None));
                }

                return Ok(());
            }
        }
    }

    //write to database
    let event_document: Document = match bson::to_bson(&event) {
        Ok(Bson::Document(event_document)) => event_document,
        _ => return Err(TipupError::from("failed to parse event document as Bson::Document")),
    };

    try!(tipup_db.collection("events").insert_one(event_document, None));
    Ok(())
}
