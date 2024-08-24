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

#[derive(Deserialize, Serialize, Default)]
struct Diff {
    intercept: Option<f64>,
    variance: Option<f64>,
    difficulty: Option<i32>,
    discrimination: Option<f64>,
    irt_loglikelihood: Option<f64>,
    irt_users: Option<i32>,
    is_experimental: Option<bool>,
}

pub async fn get_submission(pool: &Arc<Mutex<Pool>>, ctx: &serenity::Context) {
    let pool = pool.lock().await;
    let mut conn = pool.get_conn().unwrap();
    loop {
        let client = Client::builder().gzip(true).build().unwrap();
        let diff_response = client.get("https://kenkoooo.com/atcoder/resources/problem-models.json").send().await.unwrap();
        let text = diff_response.text().await.unwrap_or_default();
        let diff: BTreeMap<String, Diff> = serde_json::from_str(&text).unwrap();
        let users: Vec<(String, u64, u64)> = conn
            .query(
                "SELECT
                    users.atcoder_username,
                    notifications.submission_channel_id,
                    users.server_id
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
        let mut users_map: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
        for i in &users {
            if users_map.contains_key(&i.0) {
                users_map.get_mut(&i.0).unwrap().push((i.1, i.2));
            } else {
                users_map.insert(i.0.clone(), vec![(i.1, i.2)]);
            }
        }
        let submissions: Vec<(String, i64)> = conn.query("SELECT username,epoch_second FROM submissions").unwrap();
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
                let mut submissions: Vec<String> = conn
                    .exec(
                        "SELECT problem_id FROM submission_data WHERE user_id=:used_id",
                        params! {
                            "used_id" => &i
                        },
                    )
                    .unwrap();

                let url: String = format!(
                    "https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={}",
                    i,
                    submission_map.get(&i).unwrap()
                );

                println!("{}", url);

                let response = client.get(url).send().await.unwrap();
                let text = response.text().await.unwrap_or_default();
                let mut last: i64 = *submission_map.get(&i).unwrap();
                let json: Vec<Submission> = serde_json::from_str(&text).unwrap();
                for j in json {
                    if j.result == "AC" {
                        if submissions.contains(&j.problem_id) {
                            continue;
                        }
                        let mut diff_text = "null".to_string();
                        let diff = diff.get(&j.problem_id);
                        if let Some(diff) = diff {
                            diff_text = match diff.difficulty {
                                Some(diff) => {
                                    if diff <= 400 {
                                        ((400.0 / (f64::exp((400.0 - diff as f64) / 400.0))) as i32).to_string()
                                    } else {
                                        diff.to_string()
                                    }
                                }
                                None => "null".to_string(),
                            };
                        }
                        last = std::cmp::max(last, j.epoch_second + 1);
                        let response_ja = {
                            let response = CreateMessage::default();
                            let embed = CreateEmbed::default()
                                .title("AC Notify")
                                .description(format!(
                                    "{}は、{}の{}をACしました! Diffは{}です",
                                    j.user_id, j.contest_id, j.problem_id, diff_text
                                ))
                                .color(0x00FF00);
                            response.embed(embed)
                        };
                        let response_en = {
                            let response = CreateMessage::default();
                            let embed = CreateEmbed::default()
                                .title("AC Notify")
                                .description(format!("{} has solved {} in {}. Diff is {}", j.user_id, j.problem_id, j.contest_id, diff_text))
                                .color(0x00FF00);
                            response.embed(embed)
                        };
                        for k in users_map.get(&i).unwrap() {
                            let channel = ChannelId::new(k.0);
                            let selected_data: Vec<String> = conn
                                .exec(
                                    r"SELECT language FROM server_settings WHERE server_id=:server_id",
                                    params! {"server_id" => &k.1},
                                )
                                .unwrap();
                            let mut lang = "ja";
                            if selected_data.len() == 1 {
                                lang = selected_data[0].as_str();
                            }
                            if lang == "en" {
                                let _ = channel.send_message(&ctx.http, response_en.clone()).await;
                            } else {
                                let _ = channel.send_message(&ctx.http, response_ja.clone()).await;
                            }
                        }
                        conn.exec_drop(
                            "INSERT INTO submission_data (user_id, problem_id) VALUES (:user_id, :problem_id)",
                            params! {
                                "user_id" => j.user_id,
                                "problem_id" => &j.problem_id
                            },
                        )
                        .unwrap();
                        submissions.push(j.problem_id.clone())
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
                        "username" => i,
                    },
                )
                .unwrap();
            }
            sleep(Duration::from_millis(500)).await;
        }
        sleep(Duration::from_secs(30)).await;
    }
}
