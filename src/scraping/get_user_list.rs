use chrono::DateTime;
use mysql::prelude::*;
use mysql::*;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub async fn user_list_update(conn: &Arc<Mutex<Pool>>) {
    let pool = conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let start_time = Instant::now();
    let list: Vec<(i32, i32, i32, String, String, i8)> =
        conn.query("select * from user_ratings").unwrap();
    log::info!("get all of db {:?}", start_time.elapsed());
    let mut user_algo_history: BTreeMap<String, Vec<(i32, i32, String)>> = BTreeMap::new();
    let mut user_heuristic_history: BTreeMap<String, Vec<(i32, i32, String)>> = BTreeMap::new();
    let mut user_set = HashSet::new();
    for i in list {
        let is_algo: bool = i.5 == 0;
        user_set.insert(i.4.clone());
        let pointer: &BTreeMap<String, Vec<(i32, i32, String)>> = if is_algo {
            &user_algo_history
        } else {
            &user_heuristic_history
        };
        if pointer.contains_key(&i.4) {
            let mut vec = pointer.get(&i.4).unwrap().clone();
            let data = (i.1, i.2, i.3);
            vec.push(data);
            if is_algo {
                user_algo_history.insert(i.4, vec);
            } else {
                user_heuristic_history.insert(i.4, vec);
            }
        } else {
            let mut vec = vec![];
            let data = (i.1, i.2, i.3);
            vec.push(data);
            if is_algo {
                user_algo_history.insert(i.4, vec);
            } else {
                user_heuristic_history.insert(i.4, vec);
            }
        }
    }
    log::info!("add to BTreeMap: {:?}", start_time.elapsed());
    let contests: Vec<(String, String)> = conn
        .query("select contest_id,start_time from contests")
        .unwrap();
    let mut contest_data = BTreeMap::new();
    for (contest_id, start_time) in contests {
        contest_data.insert(
            contest_id,
            DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S%z").unwrap(),
        );
    }
    let mut transaction = conn.start_transaction(TxOpts::default()).unwrap();
    transaction
        .query_drop("delete from atcoder_user_ratings")
        .unwrap();
    for i in &user_set {
        let mut algo_aperf = 0.0;
        let mut algo_rating = 0;
        let mut heuristic_aperf = 0.0;
        let mut heuristic_rating = 0;
        let mut algo_contests = 0;
        let mut heuristic_contests = 0;
        if user_algo_history.contains_key(i) {
            let mut rating_history = user_algo_history.get(i).unwrap().clone();
            rating_history.sort_by(|a, b| {
                let duration: chrono::Duration =
                    *contest_data.get(&a.2).unwrap() - *contest_data.get(&b.2).unwrap();
                if duration.num_milliseconds() > 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            });
            let mut a = 0.0;
            let mut b = 0.0;
            let mut index = 1;
            for i in &rating_history {
                a += (i.1 as f64) * 0.9_f64.powi(index);
                b += 0.9_f64.powi(index);
                index += 1;
            }
            algo_rating = rating_history[0].0;
            algo_aperf = a / b;
            algo_contests = rating_history.len();
        }
        if user_heuristic_history.contains_key(i) {
            let mut rating_history = user_heuristic_history.get(i).unwrap().clone();
            rating_history.sort_by(|a, b| {
                let duration: chrono::Duration =
                    *contest_data.get(&a.2).unwrap() - *contest_data.get(&b.2).unwrap();
                if duration.num_milliseconds() > 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            });
            let mut a = 0.0;
            let mut b = 0.0;
            let mut index = 1;
            for i in &rating_history {
                a += (i.1 as f64) * 0.9_f64.powi(index);
                b += 0.9_f64.powi(index);
                index += 1;
            }
            heuristic_rating = rating_history[0].0;
            heuristic_aperf = a / b;
            heuristic_contests = rating_history.len();
        }
        transaction
            .exec_drop(
                "insert into atcoder_user_ratings (user_name, algo_aperf, algo_rating, algo_contests, heuristic_aperf, heuristic_rating, heuristic_contests)
                       VALUES (:user_name, :algo_aperf, :algo_rating, :algo_contests, :heuristic_aperf, :heuristic_rating, :heuristic_contests)",
                params! {
                    "user_name" => i,
                    "algo_aperf" => algo_aperf,
                    "algo_rating" => algo_rating,
                    "algo_contests" => algo_contests,
                    "heuristic_aperf" => heuristic_aperf,
                    "heuristic_rating" => heuristic_rating,
                    "heuristic_contests" => heuristic_contests,
                },
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    log::info!("add to Database: {:?}", start_time.elapsed());
}
