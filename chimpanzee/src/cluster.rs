use dbscan::{DBSCAN, SymmetricMatrix};

pub fn dbscan(flags: &Vec<Flag>, eps: f64, min_points: usize) -> (Vec<Option<usize>>, SymmetricMatrix<f64>) {
    //process results (3273 results)
    let mut dbscan = DBSCAN::new(eps, min_points);
    let mut matrix = SymmetricMatrix::<f64>::new(flags.len());
    for i in 0..flags.len()-1 {
        for j in i+1..flags.len() {
            matrix.set(i, j, compute_flag_distance(&flags[i], &flags[j]));
        }
    }

    //complete algorithm
    (dbscan.perform_clustering(&matrix).to_owned(), matrix)
}

fn compute_flag_distance(flag_one: &Flag, flag_two: &Flag) -> f64 {
    //domain
    if !flag_one.domain.eq(&flag_two.domain) {
        return 100.0;
    }

    //timestamp
    let timestamp_difference = (flag_one.timestamp - flag_two.timestamp).abs() as f64 / 3600.0; //difference in hours
    let timestamp_score = match timestamp_difference {
        0.0 ... 1.0 => (timestamp_difference + 1.00027).log2() / (24.0 as f64).log2(), //logarithmic ratio
        _ => 1.0,
    };

    //urls - TODO fuzzy match
    /*let url_score = match flag_one.url.eq(&flag_two.url) {
        true => 0.0,
        false => 1.0,
    };*/

    return timestamp_score * 100.0;
}
