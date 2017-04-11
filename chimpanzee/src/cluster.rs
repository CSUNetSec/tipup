use dbscan::{DBSCAN, SymmetricMatrix};
use compute_flag_distance;
use Flag;

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
