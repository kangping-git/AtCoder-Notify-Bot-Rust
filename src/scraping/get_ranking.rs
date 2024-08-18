use crate::utils::{
    svg::create_table::{
        self, Align, RatingCustom, RatingType, Row, TableRowsRating, TableRowsText, TextConfig,
    },
    svg_to_png::svg_to_png,
};
use serde_json;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateAttachment, CreateMessage, EditMessage,
};
use reqwest::{blocking::Client, cookie::Jar};
use tokio::sync::Mutex;

use super::ranking_types::StandingsJson;
use nutype::nutype;

#[nutype(validate(finite), derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone))]
pub struct Float(f64);

#[derive(Debug)]
struct Contest {
    contest_id: String,
    start_time: String,
    duration: i32,
    contest_type: i8,
    rating_type: i8,
    name: String,
    rating_range_end: i64,
}

#[derive(Debug, Clone)]
struct Message {
    contest_id: String,
    channel_id: i64,
    message_id: i64,
}
#[derive(Debug, Clone)]
struct User {
    atcoder_username: String,
    server_id: i64,
}

static MEMO: OnceLock<BTreeMap<Float, f64>> = OnceLock::new();

fn ordinal_suffix(n: i32) -> String {
    let suffix = match n % 10 {
        1 if n % 100 != 11 => "st",
        2 if n % 100 != 12 => "nd",
        3 if n % 100 != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

fn g(x: f64) -> f64 {
    2.0_f64.powf(x / 800.0)
}

pub async fn get_ranking(
    pool: &Arc<Mutex<Pool>>,
    cookie_store: &Arc<Jar>,
    ctx: &serenity::Context,
) {
    let mut memo_data = Clone::clone(MEMO.get_or_init(|| {
        let map: BTreeMap<Float, f64> = BTreeMap::new();
        map
    }));
    let pool_temp = pool.lock().await;
    let mut conn = pool_temp.get_conn().unwrap();
    let contests: Vec<Contest> = conn
        .query_map(
            "select contest_id,start_time,duration,contest_type,rating_type,name,rating_range_end from contests",
            |(contest_id, start_time, duration, contest_type, rating_type, name,rating_range_end)| Contest {
                contest_id,
                start_time,
                duration,
                contest_type,
                rating_type,
                name,rating_range_end
            },
        )
        .unwrap();
    let contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| {
            let start_time =
                chrono::DateTime::parse_from_str(&contest.start_time, "%Y-%m-%d %H:%M:%S%z")
                    .unwrap();
            let offset = chrono::Duration::minutes(contest.duration as i64);
            let end_time = start_time + offset;
            start_time <= chrono::Local::now() && chrono::Local::now() <= end_time
            // chrono::Local::now() <= end_time
        })
        .collect();
    let messages = conn
        .query_map(
            "SELECT contest_id, channel_id, message_id from messages",
            |(contest_id, channel_id, message_id)| Message {
                contest_id,
                channel_id,
                message_id,
            },
        )
        .unwrap();
    let mut contest_message_map: BTreeMap<String, Vec<Message>> = BTreeMap::new();
    for i in messages {
        let message = i.clone();
        let mut vec = contest_message_map
            .remove(&i.contest_id)
            .unwrap_or_default();
        vec.push(message);
        contest_message_map.insert(i.contest_id, vec);
    }

    let users = conn
        .query_map(
            "SELECT atcoder_username, server_id from users",
            |(atcoder_username, server_id)| User {
                atcoder_username,
                server_id,
            },
        )
        .unwrap();
    let mut contest_users_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for user in users {
        let user = user.clone();
        let mut vec = contest_users_map
            .remove(&user.server_id.to_string())
            .unwrap_or_default();
        vec.insert(user.atcoder_username.to_lowercase());
        contest_users_map.insert(user.server_id.to_string(), vec);
    }

    let channels: Vec<(String, String)> = conn
        .query("SELECT contest_channel_id,server_id FROM notifications")
        .unwrap();

    let users: Vec<(String, f64, f64, i32, i32)> = conn
        .query("SELECT user_name,algo_aperf,heuristic_aperf,algo_contests,heuristic_contests FROM atcoder_user_ratings")
        .unwrap();
    let users_to_aperf: BTreeMap<String, (f64, f64, i32, i32)> = users
        .iter()
        .map(
            |(user_name, algo_aperf, heuristic_aperf, algo_contests, heuristic_contests)| {
                (
                    user_name.clone().to_lowercase(),
                    (
                        *algo_aperf,
                        *heuristic_aperf,
                        *algo_contests,
                        *heuristic_contests,
                    ),
                )
            },
        )
        .collect();

    let client = Client::builder()
        .cookie_store(true)
        .cookie_provider(Arc::clone(cookie_store))
        .build()
        .unwrap();
    for i in &contests {
        let url = format!("https://{}/standings/json", i.contest_id);
        let json = client.get(url).send().unwrap().text().unwrap_or_default();
        let data: StandingsJson = serde_json::from_str(&json).unwrap_or_default();
        let empty_set: BTreeSet<String> = BTreeSet::new();
        for (channel_id, server_id) in &channels {
            if channel_id != "null" {
                let user_list = contest_users_map
                    .get(server_id)
                    .unwrap_or(Clone::clone(&&empty_set));
                let mut ranks = vec![];
                let mut server_ranks = vec![];
                let mut users_list = vec![];
                let mut perf_list = vec![];
                let mut old_rate_list = vec![];
                let mut new_rate_list = vec![];
                let mut rate_diff_list = vec![];
                let mut last_rank = 0;
                let mut server_rank = 1;
                let mut rank_people = 0;
                let mut rank_map: BTreeMap<i32, i32> = BTreeMap::new();
                let mut rank_people_map: BTreeMap<i32, i32> = BTreeMap::new();
                let mut rank = 1;
                let mut now_rank = 1;
                let mut on_rank_people = 0;
                for users in &data.StandingsData {
                    if users.IsRated {
                        if users.Rank != now_rank {
                            rank += on_rank_people;
                            on_rank_people = 0;
                            now_rank = users.Rank;
                        }
                        on_rank_people += 1;
                        rank_map.insert(users.Rank, rank);
                        rank_people_map.insert(users.Rank, on_rank_people);
                    } else {
                        rank_map.insert(users.Rank, rank);
                    }
                }
                for users in &data.StandingsData {
                    if user_list.contains(&users.UserScreenName.to_lowercase()) {
                        if last_rank != users.Rank {
                            server_rank += rank_people;
                            last_rank = users.Rank;
                            rank_people = 0;
                        }
                        let mut rank = (*rank_map.get(&users.Rank).unwrap() as f64)
                            + ((rank_people_map.get(&users.Rank).unwrap() - 1) as f64) / 2.0;
                        if rank_people_map.get(&users.Rank).unwrap() == &0 {
                            rank = *rank_map.get(&users.Rank).unwrap() as f64;
                        }
                        let mut r = 10000.0;
                        let mut l = -10000.0;
                        while r - l > 0.1 {
                            let x = (r + l) / 2.0;
                            let mut sum = 0.0;
                            let contains_key = memo_data.contains_key(&Float::try_new(x).unwrap());
                            if !contains_key {
                                for j in &data.StandingsData {
                                    if j.IsRated {
                                        let aperf = users_to_aperf
                                            .get(&j.UserScreenName.to_lowercase())
                                            .unwrap_or(match i.rating_type {
                                                2 => &(1200.0, 1200.0, 0, 0),
                                                1 => &(1000.0, 1000.0, 0, 0),
                                                _ => &(800.0, 800.0, 0, 0),
                                            });
                                        let aperf = match i.contest_type {
                                            0 => match aperf.2 {
                                                0 => match i.rating_type {
                                                    2 => 1200.0,
                                                    1 => 1000.0,
                                                    _ => 800.0,
                                                },
                                                _ => aperf.0,
                                            },
                                            _ => match aperf.3 {
                                                0 => match i.rating_type {
                                                    2 => 1200.0,
                                                    1 => 1000.0,
                                                    _ => 800.0,
                                                },
                                                _ => aperf.1,
                                            },
                                        };
                                        sum += 1.0 / (1.0 + 6.0_f64.powf((x - aperf) / 400.0));
                                    }
                                }
                                memo_data.insert(Float::try_new(x).unwrap(), sum);
                            } else {
                                sum = *memo_data.get(&Float::try_new(x).unwrap()).unwrap();
                            }
                            if sum < rank - 0.5 {
                                r = x;
                            } else {
                                l = x;
                            }
                        }
                        let mut perf = (r + l) / 2.0;
                        if perf <= 400.0 {
                            perf = 400.0 / (f64::exp((400.0 - perf) / 400.0))
                        }

                        if i.rating_range_end < 0 && perf >= i.rating_range_end as f64 + 401.0 {
                            perf = i.rating_range_end as f64 + 401.0
                        }

                        let performance_list:Vec<i32> = conn.exec(
                                "SELECT
                                           LEAST(contests.rating_range_end + 401,user_ratings.performance)
                                       FROM
                                           user_ratings
                                       JOIN
                                           contests
                                       ON
                                           contests.contest_id = user_ratings.contest
                                       WHERE
                                           user_name = :user_name AND type = :type",
                                params! {
                                    "user_name" => &users.UserScreenName,
                                    "type" => i.contest_type
                                },
                            ).unwrap();
                        let mut up = 0.9 * 2.0 * g(perf);
                        let mut down = 0.9;
                        let mut count = 2;
                        for i in &performance_list {
                            up += 2.0 * g(*i as f64) * 0.9_f64.powf(count as f64);
                            down += 0.9_f64.powf(count as f64);
                            count += 1;
                        }
                        let mut rate = f64::log2(up / down) * 800.0
                            - ((f64::sqrt(
                                1.0 - 0.81_f64.powi((performance_list.len() + 1) as i32),
                            ) / (1.0 - 0.9_f64.powi((performance_list.len() + 1) as i32)))
                                - 1.0)
                                / (f64::sqrt(19.0) - 1.0)
                                * 1200.0;
                        if rate <= 400.0 {
                            rate = 400.0 / (f64::exp((400.0 - rate) / 400.0))
                        }

                        new_rate_list.push(RatingType::Custom(RatingCustom {
                            rating: rate as i32,
                            title: (rate as i32).to_string(),
                        }));
                        old_rate_list.push(RatingType::Custom(RatingCustom {
                            rating: users.Rating,
                            title: users.Rating.to_string(),
                        }));

                        rate_diff_list.push(TextConfig {
                            value: if rate as i32 - users.Rating > 0 {
                                format!("+{}", rate as i32 - users.Rating)
                            } else {
                                (rate as i32 - users.Rating).to_string()
                            },
                            color: match rate as i32 - users.Rating {
                                x if x > 0 => "red",
                                x if x < 0 => "Aquamarine",
                                _ => "white",
                            }
                            .to_string(),
                        });

                        perf_list.push(RatingType::Custom(RatingCustom {
                            rating: perf as i32,
                            title: (perf as i32).to_string(),
                        }));
                        ranks.push(TextConfig {
                            value: ordinal_suffix(users.Rank),
                            color: match users.Rank {
                                1 => "#FFD700",
                                2 => "#C0C0C0",
                                3 => "#FFD700",
                                _ => "white",
                            }
                            .to_string(),
                        });
                        server_ranks.push(TextConfig {
                            value: ordinal_suffix(server_rank),
                            color: match server_rank {
                                1 => "#FFD700",
                                2 => "#C0C0C0",
                                3 => "#FFD700",
                                _ => "white",
                            }
                            .to_string(),
                        });
                        rank_people += 1;

                        users_list.push(RatingType::UserRating(create_table::UserRating {
                            username: users.UserScreenName.clone(),
                            contest_type: match i.rating_type {
                                0 => super::contest_type::ContestType::Algorithm,
                                _ => super::contest_type::ContestType::Heuristic,
                            },
                        }));
                    }
                }
                let rows = vec![
                    Row::Text(TableRowsText {
                        title: "全体".to_string(),
                        width: 300,
                        align: Align::Start,
                        data: ranks,
                    }),
                    Row::Text(TableRowsText {
                        title: "鯖内".to_string(),
                        width: 300,
                        align: Align::Start,
                        data: server_ranks,
                    }),
                    Row::Rating(TableRowsRating {
                        title: "ユーザー".to_string(),
                        width: 1300,
                        data: users_list,
                    }),
                    Row::Rating(TableRowsRating {
                        title: "perf".to_string(),
                        width: 300,
                        data: perf_list,
                    }),
                    Row::Rating(TableRowsRating {
                        title: "Old".to_string(),
                        width: 300,
                        data: old_rate_list,
                    }),
                    Row::Rating(TableRowsRating {
                        title: "New".to_string(),
                        width: 300,
                        data: new_rate_list,
                    }),
                    Row::Text(TableRowsText {
                        title: "Diff".to_string(),
                        width: 300,
                        align: Align::Middle,
                        data: rate_diff_list,
                    }),
                ];
                let svg = create_table::create_table(
                    &Arc::new(Mutex::new(pool_temp.clone())),
                    format!("{} サーバー内ランキング", i.name),
                    rows,
                )
                .await;
                let channel = ChannelId::new(channel_id.parse::<u64>().unwrap());
                let response = {
                    let mut message = CreateMessage::new();
                    message = message.add_file(CreateAttachment::bytes(
                        svg_to_png(
                            svg.svg.as_str(),
                            svg.width as u32,
                            svg.height as u32,
                            1.0,
                            1.0,
                        ),
                        "ranking.png",
                    ));
                    message
                };
                if contest_message_map.contains_key(&i.contest_id) {
                    let message_id: Vec<&Message> = contest_message_map
                        .get(&i.contest_id)
                        .unwrap()
                        .iter()
                        .filter(|x| x.channel_id.to_string() == channel_id.clone())
                        .collect();
                    if !message_id.is_empty() {
                        let channel = ChannelId::new(message_id[0].channel_id as u64);
                        let _ = channel
                            .edit_message(
                                ctx.http.clone(),
                                message_id[0].message_id as u64,
                                EditMessage::default().new_attachment(CreateAttachment::bytes(
                                    svg_to_png(
                                        svg.svg.as_str(),
                                        svg.width as u32,
                                        svg.height as u32,
                                        1.0,
                                        1.0,
                                    ),
                                    "ranking.png",
                                )),
                            )
                            .await;
                    }
                } else {
                    let message = channel.send_message(&ctx.http, response).await;
                    if let Ok(message_id) = message {
                        conn.exec_drop(
                                "INSERT INTO messages (contest_id, channel_id, message_id) VALUES (:contest_id, :channel_id, :message_id)",
                                params! {
                                    "contest_id" => &i.contest_id,
                                    "channel_id" => channel_id.parse::<i64>().unwrap(),
                                    "message_id" => message_id.id.get() as i64
                                },
                            ).unwrap();
                    }
                }
            }
        }
    }
}
