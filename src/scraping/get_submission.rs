use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, ChannelId, CreateEmbed, CreateMessage};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Mutex, time::sleep};

#[derive(Deserialize, Serialize)]
struct Submission {
    id: i64,
    epoch_second: i64,
    problem_id: String,
    contest_id: String,
    user_id: String,
    language: String,
    point: f64,
    length: i32,
    result: String,
    execution_time: Option<i32>,
}

pub async fn get_submission(pool: &Arc<Mutex<Pool>>, ctx: &serenity::Context) {
    let pool = pool.lock().await;
    let mut conn = pool.get_conn().unwrap();
    loop {
        let users: Vec<(String, u64)> = conn
            .query(
                "SELECT
                    users.atcoder_username,
                    notifications.submission_channel_id
                FROM
                    users
                JOIN
                    notifications
                ON
                    users.server_id = notifications.server_id
                WHERE
                    notifications.submission_channel_id IS NOT NULL",
            )
            .unwrap();
        let mut users_map: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for i in &users {
            if users_map.contains_key(&i.0) {
                users_map.get_mut(&i.0).unwrap().push(i.1);
            } else {
                users_map.insert(i.0.clone(), vec![i.1]);
            }
        }
        let submissions: Vec<(String, i64)> = conn
            .query("SELECT username,epoch_second FROM submissions")
            .unwrap();
        let mut submission_map = BTreeMap::new();
        for i in submissions {
            submission_map.insert(i.0, i.1);
        }
        let mut user_set = BTreeSet::new();
        for i in users {
            user_set.insert(i.0);
        }
        for i in user_set {
            if submission_map.contains_key(&i) {
                let url: String = format!(
                    "https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={}",
                    i,submission_map.get(&i).unwrap()
                );
                log::info!("{}", url);
                let client = Client::builder().gzip(true).build().unwrap();
                let response = client.get(url).send().await.unwrap();
                let text = response.text().await.unwrap_or_default();
                log::info!("{}", text);
                let mut last: i64 = *submission_map.get(&i).unwrap();
                let json: Vec<Submission> = serde_json::from_str(&text).unwrap();
                for j in json {
                    if j.result == "AC" {
                        last = std::cmp::max(last, j.epoch_second + 1);
                        let response = {
                            let response = CreateMessage::default();
                            let embed = CreateEmbed::default()
                                .title("AC Notify")
                                .description(format!(
                                    "{} has solved {} in {}.",
                                    j.user_id, j.problem_id, j.contest_id
                                ))
                                .color(0x00FF00);
                            response.embed(embed)
                        };
                        for k in users_map.get(&i).unwrap() {
                            let channel = ChannelId::new(*k);
                            let _ = channel.send_message(&ctx.http, response.clone()).await;
                        }
                    }
                }
                conn.exec_drop(
                    "UPDATE submissions SET epoch_second=:epoch_second WHERE username=:username",
                    params! {
                        "epoch_second" => last,
                        "username" => &i
                    },
                )
                .unwrap();
            } else {
                conn.exec_drop(
                    "INSERT INTO submissions (username) VALUES (:username)",
                    params! {
                        "username" => i
                    },
                )
                .unwrap();
            }
            sleep(Duration::from_millis(500)).await;
        }
        sleep(Duration::from_secs(30)).await;
    }
}
