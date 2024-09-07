use chrono::DateTime;
use fontdb::{Database, Query, Source};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use fontdue::Font;
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateAttachment, CreateMessage};
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::utils::svg::create_table::{self, Align, RatingType, Row, TableRowsRating, TableRowsText, TextConfig, Title};
use crate::utils::svg::create_user_rating::Theme;
use crate::utils::svg_to_png::svg_to_png;

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

#[derive(Debug, Clone)]
struct User {
    atcoder_username: String,
    server_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

fn ordinal_suffix(n: i32) -> String {
    let suffix = match n % 10 {
        1 if n % 100 != 11 => "st",
        2 if n % 100 != 12 => "nd",
        3 if n % 100 != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

pub async fn get_ratings(cookie_store: &Arc<Jar>, conn_raw: &Arc<Mutex<Pool>>, ctx: &Context, get_all: bool) -> Result<()> {
    let pool = conn_raw.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let contests: Vec<(String, i8, i32, bool, String, i32)> = conn
        .query(r"select contest_id,contest_type,rating_range_end,get_user_ratings_flag,start_time,duration from contests where rating_range_end>=0")
        .unwrap();
    let mut contests: Vec<&(String, i8, i32, bool, String, i32)> = contests
        .iter()
        .filter(|contest| {
            let start_time = chrono::DateTime::parse_from_str(&contest.4, "%Y-%m-%d %H:%M:%S%z").unwrap();
            let offset = chrono::Duration::minutes(contest.5 as i64);
            let end_time = start_time + offset;
            chrono::Local::now() >= end_time
        })
        .collect();

    contests.sort_by(|a, b| {
        let duration: chrono::Duration =
            DateTime::parse_from_str(&a.4, "%Y-%m-%d %H:%M:%S%z").unwrap() - DateTime::parse_from_str(&b.4, "%Y-%m-%d %H:%M:%S%z").unwrap();
        if duration.num_milliseconds() > 0 {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });

    let mut user_history: Vec<UserRatings> = vec![];

    let mut algo_rating_cache: BTreeMap<String, Vec<UserRatingDataFromAtCoder>> = BTreeMap::new();
    let mut heuristic_rating_cache: BTreeMap<String, Vec<UserRatingDataFromAtCoder>> = BTreeMap::new();

    let client = Client::builder().cookie_store(true).cookie_provider(Arc::clone(cookie_store)).build().unwrap();

    let users = conn
        .query_map("SELECT atcoder_username, server_id from users", |(atcoder_username, server_id)| User {
            atcoder_username,
            server_id,
        })
        .unwrap();

    let mut contest_users_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut users_set = BTreeSet::new();
    for user in users {
        let user = user.clone();
        let mut vec = contest_users_map.remove(&user.server_id.to_string()).unwrap_or_default();
        vec.insert(user.atcoder_username.to_lowercase());
        contest_users_map.insert(user.server_id.to_string(), vec);
        users_set.insert(user.atcoder_username.to_lowercase());
    }

    let mut rating_data: BTreeMap<String, ResultData> = BTreeMap::new();

    let channels: Vec<(String, String)> = conn.query("SELECT contest_channel_id,server_id FROM notifications").unwrap();

    let mut contests_list: Vec<String> = vec![];
    let mut is_first = true;

    for (contest_id, contest_type, rating_range_end, is_already_get, _, _) in contests {
        if !get_all && *is_already_get {
            continue;
        }
        if !get_all && !is_first {
            continue;
        }
        sleep(Duration::from_millis(500)).await;
        log::info!("get all user rating: {}", contest_id);
        let url = format!("https://{}/results/json", contest_id);
        let response = client.get(url).send().unwrap().text().unwrap_or_default();
        let response_text = response.trim();
        let data: Vec<ResultData> = serde_json::from_str(response_text).unwrap();
        if !data.is_empty() {
            is_first = false;
            contests_list.push(contest_id.clone());
            for mut i in data {
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
                                rating_history.clone_from(algo_rating_cache.get(&i.UserScreenName).unwrap());
                            }
                        } else {
                            has_cache = heuristic_rating_cache.contains_key(&i.UserScreenName);
                            if has_cache {
                                rating_history.clone_from(heuristic_rating_cache.get(&i.UserScreenName).unwrap());
                            }
                        }
                        if !has_cache {
                            let algo_flag = if *contest_type == 0 { "contestType=algo" } else { "contestType=heuristic" };
                            sleep(Duration::from_millis(500)).await;
                            println!("get inner performance: {}", i.UserScreenName);
                            let user_rating_page = format!("https://atcoder.jp/users/{}/history/json?{}", &i.UserScreenName, algo_flag);
                            println!("{user_rating_page}");
                            let response = client.get(user_rating_page).send().unwrap().text().unwrap();
                            let response_text = response.trim();
                            let response_json = serde_json::from_str(response_text);
                            match response_json {
                                Ok(response_json) => {
                                    rating_history = response_json;
                                    if *contest_type == 0 {
                                        algo_rating_cache.insert(i.UserScreenName.clone(), rating_history.clone());
                                    } else {
                                        heuristic_rating_cache.insert(i.UserScreenName.clone(), rating_history.clone());
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
                        println!("user {}'s innerPerf is {performance}. is_delete flag is {is_delete}", i.UserScreenName)
                    }
                    if !is_delete {
                        user_history.push(UserRatings {
                            performance,
                            contest_type: *contest_type,
                            rating: new_rating,
                            contest: contest_id.clone(),
                            user_name: i.UserScreenName.clone(),
                        })
                    }
                    if users_set.contains(&i.UserScreenName.to_lowercase()) {
                        i.Performance = performance;
                        rating_data.insert(i.UserScreenName.to_lowercase().clone(), i.clone());
                    }
                }
            }
        }
    }
    if !contests_list.is_empty() && !get_all {
        let mut db = Database::new();
        db.load_system_fonts();

        let query = Query {
            families: &[fontdb::Family::Name("Lato")],
            weight: fontdb::Weight::BOLD,
            ..Default::default()
        };
        let id = db.query(&query).unwrap();
        let face = db.face(id).unwrap();
        let font_data = match &face.source {
            Source::Binary(data) => data.as_ref().as_ref(),
            Source::File(path) => &std::fs::read(path).unwrap_or_else(|_| panic!("Error loading font data from file: {:?}", path)),
            err => panic!("Error loading font data. {:?}", err),
        };

        let font = Font::from_bytes(font_data, fontdue::FontSettings::default()).expect("Error loading font");
        let scale = 70.0;

        for i in channels {
            let users = contest_users_map.get(&i.1).unwrap();
            let mut user_data = vec![];
            for user in users {
                if let Some(data) = rating_data.get(user) {
                    user_data.push(data.clone());
                }
            }
            user_data.sort_by(|a, b| a.Place.cmp(&b.Place));
            let mut server_ranks = vec![];
            let mut server_rank = 1;
            let mut last_rank = 1;
            let mut rank_people = 1;
            for j in &user_data {
                server_ranks.push(server_rank);
                if last_rank != j.Place {
                    server_rank += rank_people;
                    rank_people = 1;
                    last_rank = j.Place;
                } else {
                    rank_people += 1;
                }
            }

            let mut all_rank_vec = vec![];
            let mut server_rank_vec = vec![];
            let mut user_name = vec![];
            let mut old_rating = vec![];
            let mut new_rating = vec![];
            let mut diff = vec![];
            let mut rated = vec![];
            let mut performance = vec![];
            let mut user_width = 0;
            for (i, result_data) in user_data.iter().enumerate() {
                all_rank_vec.push(TextConfig {
                    value: ordinal_suffix(result_data.Place),
                    color: match server_rank {
                        1 => "#FFD700",
                        2 => "#C0C0C0",
                        3 => "#CD7F32",
                        _ => "white",
                    }
                    .to_string(),
                });
                server_rank_vec.push(TextConfig {
                    value: ordinal_suffix(server_ranks[i]),
                    color: match server_rank {
                        1 => "#FFD700",
                        2 => "#C0C0C0",
                        3 => "#CD7F32",
                        _ => "white",
                    }
                    .to_string(),
                });
                user_name.push(RatingType::Custom(create_table::RatingCustom {
                    has_bronze: false,
                    rating: result_data.NewRating,
                    title: result_data.UserScreenName.clone(),
                    color_theme: Theme::Dark,
                }));
                old_rating.push(RatingType::Custom(create_table::RatingCustom {
                    has_bronze: false,
                    rating: result_data.OldRating,
                    title: result_data.OldRating.to_string(),
                    color_theme: Theme::Dark,
                }));
                new_rating.push(RatingType::Custom(create_table::RatingCustom {
                    has_bronze: false,
                    rating: result_data.NewRating,
                    title: result_data.NewRating.to_string(),
                    color_theme: Theme::Dark,
                }));
                performance.push(RatingType::Custom(create_table::RatingCustom {
                    has_bronze: false,
                    rating: result_data.Performance,
                    title: if result_data.Performance == 0 {
                        "-".to_string()
                    } else {
                        result_data.Performance.to_string()
                    },
                    color_theme: Theme::Dark,
                }));
                diff.push(TextConfig {
                    value: if result_data.NewRating - result_data.OldRating > 0 {
                        format!("+{}", result_data.NewRating - result_data.OldRating)
                    } else {
                        (result_data.NewRating - result_data.OldRating).to_string()
                    },
                    color: match result_data.NewRating - result_data.OldRating {
                        x if x > 0 => "red",
                        x if x < 0 => "Aquamarine",
                        _ => "white",
                    }
                    .to_string(),
                });
                rated.push(TextConfig {
                    value: if result_data.IsRated { "Yes".to_string() } else { "No".to_string() },
                    color: if result_data.IsRated { "white" } else { "gray" }.to_string(),
                });

                let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                layout.append(&[font.clone()], &TextStyle::new(&result_data.UserScreenName, scale, 0));

                let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32);
                user_width = user_width.max(width as i32);
            }
            let rows = vec![
                Row::Text(TableRowsText {
                    title: Title::Text("All".to_string()),
                    width: 300,
                    align: Align::End,
                    data: all_rank_vec,
                }),
                Row::Text(TableRowsText {
                    title: Title::Text("Server".to_string()),
                    width: 300,
                    align: Align::End,
                    data: server_rank_vec,
                }),
                Row::Rating(TableRowsRating {
                    title: Title::Text("User".to_string()),
                    width: (user_width + 120).max(300),
                    data: user_name,
                }),
                Row::Rating(TableRowsRating {
                    title: Title::Text("Perf".to_string()),
                    width: 300,
                    data: performance,
                }),
                Row::Rating(TableRowsRating {
                    title: Title::Text("Old".to_string()),
                    width: 300,
                    data: old_rating,
                }),
                Row::Rating(TableRowsRating {
                    title: Title::Text("New".to_string()),
                    width: 300,
                    data: new_rating,
                }),
                Row::Text(TableRowsText {
                    title: Title::Text("Diff".to_string()),
                    width: 300,
                    align: Align::Middle,
                    data: diff,
                }),
                Row::Text(TableRowsText {
                    title: Title::Text("Rated".to_string()),
                    width: 300,
                    align: Align::Middle,
                    data: rated,
                }),
            ];

            let svg = create_table::create_table(
                &Arc::new(Mutex::new(pool.clone())),
                format!("レーティング更新:{}", user_data[0].ContestName),
                rows,
            )
            .await;
            let response = {
                let mut message = CreateMessage::new();
                message = message.content("レーティングが更新されました").add_file(CreateAttachment::bytes(
                    svg_to_png(svg.svg.as_str(), svg.width as u32, svg.height as u32, 1.0, 1.0),
                    "ranking.png",
                ));
                message
            };

            let channel = ChannelId::new(i.0.parse::<u64>().unwrap());
            channel.send_message(ctx.http(), response).await.unwrap_or_default();
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
        get_user_list::user_list_update(conn_raw, ctx).await.unwrap_or_default();
    }
    Ok(())
}
