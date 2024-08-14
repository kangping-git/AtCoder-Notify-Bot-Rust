use chrono::{self, DateTime};
use mysql::prelude::*;
use mysql::*;
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::get_user_list;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct UserRatings {
    rating: i32,
    performance: i32,
    contest: String,
    user_name: String,
    contest_type: i8,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct ResultData {
    IsRated: bool,
    Place: i32,
    OldRating: i32,
    NewRating: i32,
    Performance: i32,
    ContestName: String,
    ContestNameEn: String,
    ContestScreenName: String,
    EndTime: String,
    ContestType: i32,
    UserName: String,
    UserScreenName: String,
    Country: String,
    Affiliation: String,
    Rating: i32,
    Competitions: i32,
    AtCoderRank: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct UserRatingDataFromAtCoder {
    IsRated: bool,
    Place: i32,
    OldRating: i32,
    NewRating: i32,
    Performance: i32,
    InnerPerformance: i32,
    ContestScreenName: String,
    ContestName: String,
    ContestNameEn: String,
    EndTime: String,
}

pub async fn get_ratings(cookie_store: &Arc<Jar>, conn_raw: &Arc<Mutex<Pool>>, get_all: bool) {
    let pool = conn_raw.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let contests: Vec<(String, i8, i32,bool,String,i32)> = conn
        .query(r"select contest_id,contest_type,rating_range_end,get_user_ratings_flag,start_time,duration from contests where rating_range_end>=0")
        .unwrap();
    let mut contests: Vec<&(String, i8, i32, bool, String, i32)> = contests
        .iter()
        .filter(|contest| {
            let start_time =
                chrono::DateTime::parse_from_str(&contest.4, "%Y-%m-%d %H:%M:%S%z").unwrap();
            let offset = chrono::Duration::minutes(contest.5 as i64);
            let end_time = start_time + offset;
            chrono::Local::now() >= end_time
        })
        .collect();

    contests.sort_by(|a, b| {
        let duration: chrono::Duration = DateTime::parse_from_str(&a.4, "%Y-%m-%d %H:%M:%S%z")
            .unwrap()
            - DateTime::parse_from_str(&b.4, "%Y-%m-%d %H:%M:%S%z").unwrap();
        if duration.num_milliseconds() > 0 {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });

    let mut user_history: Vec<UserRatings> = vec![];

    let mut algo_rating_cache: BTreeMap<String, Vec<UserRatingDataFromAtCoder>> = BTreeMap::new();
    let mut heuristic_rating_cache: BTreeMap<String, Vec<UserRatingDataFromAtCoder>> =
        BTreeMap::new();

    let client = Client::builder()
        .cookie_store(true)
        .cookie_provider(Arc::clone(cookie_store))
        .build()
        .unwrap();

    let mut contests_list: Vec<String> = vec![];

    for (contest_id, contest_type, rating_range_end, is_already_get, _, _) in contests {
        if !get_all && *is_already_get {
            continue;
        }
        sleep(Duration::from_millis(1000)).await;
        log::info!("get all user rating: {}", contest_id);
        let url = format!("https://{}/results/json", contest_id);
        let response = client.get(url).send().unwrap().text().unwrap_or_default();
        let response_text = response.trim();
        if !response_text.is_empty() {
            contests_list.push(contest_id.clone());
            let data: Vec<ResultData> = serde_json::from_str(response_text).unwrap();
            for i in data {
                if i.IsRated {
                    let mut is_delete = false;
                    let new_rating = i.NewRating;
                    let mut performance = i.Performance;
                    if i.Performance >= rating_range_end + 401 {
                        let mut rating_history = vec![];
                        let has_cache;
                        if *contest_type == 0 {
                            has_cache = algo_rating_cache.contains_key(&i.UserScreenName);
                            if has_cache {
                                rating_history
                                    .clone_from(algo_rating_cache.get(&i.UserScreenName).unwrap());
                            }
                        } else {
                            has_cache = heuristic_rating_cache.contains_key(&i.UserScreenName);
                            if has_cache {
                                rating_history.clone_from(
                                    heuristic_rating_cache.get(&i.UserScreenName).unwrap(),
                                );
                            }
                        }
                        if !has_cache {
                            let algo_flag = if *contest_type == 0 {
                                "contestType=algo"
                            } else {
                                "contestType=heuristic"
                            };
                            sleep(Duration::from_millis(1000)).await;
                            log::info!("get inner performance: {}", i.UserScreenName);
                            let user_rating_page = format!(
                                "https://atcoder.jp/users/{}/history/json?{}",
                                &i.UserScreenName, algo_flag
                            );
                            log::info!("{user_rating_page}");
                            let response =
                                client.get(user_rating_page).send().unwrap().text().unwrap();
                            let response_text = response.trim();
                            let response_json = serde_json::from_str(response_text);
                            match response_json {
                                Ok(response_json) => {
                                    rating_history = response_json;
                                    if *contest_type == 0 {
                                        algo_rating_cache.insert(
                                            i.UserScreenName.clone(),
                                            rating_history.clone(),
                                        );
                                    } else {
                                        heuristic_rating_cache.insert(
                                            i.UserScreenName.clone(),
                                            rating_history.clone(),
                                        );
                                    }
                                }
                                Err(_) => is_delete = true,
                            }
                        }
                        for i in &rating_history {
                            if i.ContestScreenName == contest_id.clone() {
                                performance = i.InnerPerformance
                            }
                        }
                        log::info!(
                            "user {}'s innerPerf is {performance}. is_delete flag is {is_delete}",
                            i.UserScreenName
                        )
                    }
                    if !is_delete {
                        user_history.push(UserRatings {
                            performance,
                            contest_type: *contest_type,
                            rating: new_rating,
                            contest: contest_id.clone(),
                            user_name: i.UserScreenName,
                        })
                    }
                }
            }
        }
    }

    let mut transaction = conn.start_transaction(TxOpts::default()).unwrap();

    transaction
        .exec_batch(
            r"
                INSERT INTO user_ratings (user_name, rating, performance, contest, type)
                VALUES (:user_name, :rating, :performance, :contest, :type)",
            user_history.iter().map(|ur| {
                params! {
                    "rating" => ur.rating,
                    "performance" => ur.performance,
                    "contest" => &ur.contest,
                    "user_name" => &ur.user_name,
                    "type" => &ur.contest_type
                }
            }),
        )
        .unwrap();

    for contest_id in &contests_list {
        transaction
            .exec_drop(
                r"UPDATE contests set get_user_ratings_flag=1 where contest_id=:contest_id",
                params! {"contest_id" => &contest_id},
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    if !contests_list.is_empty() {
        let conn = Arc::new(Mutex::new(pool.clone()));
        let conn_clone = Arc::clone(&conn);
        thread::spawn(move || async move {
            get_user_list::user_list_update(&conn_clone).await;
        });
    }
}
