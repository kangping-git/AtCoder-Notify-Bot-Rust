use crate::utils::{
    svg::{
        create_table::{self, Align, RatingCustom, RatingType, Row, TableRowsRating, TableRowsText, TextConfig, Title},
        create_user_rating::Theme,
    },
    svg_to_png::svg_to_png,
};
use core::f64;
use serde_json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, ChannelId, CreateAttachment, CreateMessage, EditMessage};
use reqwest::{blocking::Client, cookie::Jar};
use tokio::sync::Mutex;

use super::{diff, ranking_types::StandingsJson};
use fontdb::{Database, Query, Source};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use fontdue::Font;
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

const S: f64 = 724.4744301;
const R: f64 = 0.8271973364;

fn ordinal_suffix(n: i32) -> String {
    let suffix = match n % 10 {
        1 if n % 100 != 11 => "st",
        2 if n % 100 != 12 => "nd",
        3 if n % 100 != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

pub async fn get_ranking(pool: &Arc<Mutex<Pool>>, cookie_store: &Arc<Jar>, ctx: &serenity::Context) -> Result<()> {
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

    let pool_temp = pool.lock().await;
    let mut conn = pool_temp.get_conn().unwrap();
    let contests: Vec<Contest> = conn
        .query_map(
            "select contest_id,start_time,duration,contest_type,rating_type,name,rating_range_end from contests",
            |(contest_id, start_time, duration, contest_type, rating_type, name, rating_range_end)| Contest {
                contest_id,
                start_time,
                duration,
                contest_type,
                rating_type,
                name,
                rating_range_end,
            },
        )
        .unwrap();
    let contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| {
            let start_time = chrono::DateTime::parse_from_str(&contest.start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
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
        let mut vec = contest_message_map.remove(&i.contest_id).unwrap_or_default();
        vec.push(message);
        contest_message_map.insert(i.contest_id, vec);
    }

    let users = conn
        .query_map("SELECT atcoder_username, server_id from users", |(atcoder_username, server_id)| User {
            atcoder_username,
            server_id,
        })
        .unwrap();
    let mut contest_users_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for user in users {
        let user = user.clone();
        let mut vec = contest_users_map.remove(&user.server_id.to_string()).unwrap_or_default();
        vec.insert(user.atcoder_username.to_lowercase());
        contest_users_map.insert(user.server_id.to_string(), vec);
    }

    let channels: Vec<(String, String)> = conn.query("SELECT contest_channel_id,server_id FROM notifications").unwrap();

    let users: Vec<(String, f64, f64, i32, i32)> =
        conn.query("SELECT user_name,algo_aperf,heuristic_aperf,algo_contests,heuristic_contests FROM atcoder_user_ratings").unwrap();
    let users_to_aperf: BTreeMap<String, (f64, f64, i32, i32)> = users
        .iter()
        .map(|(user_name, algo_aperf, heuristic_aperf, algo_contests, heuristic_contests)| {
            (
                user_name.clone().to_lowercase(),
                (*algo_aperf, *heuristic_aperf, *algo_contests, *heuristic_contests),
            )
        })
        .collect();

    let client = Client::builder().cookie_store(true).cookie_provider(Arc::clone(cookie_store)).build().unwrap();
    for i in &contests {
        let mut memo_data: BTreeMap<Float, f64> = BTreeMap::new();
        let url = format!("https://{}/standings/json", i.contest_id);
        let json = client.get(url).send().unwrap().text().unwrap_or_default();
        let data: StandingsJson = serde_json::from_str(&json).unwrap_or_default();
        let mut rank_map: BTreeMap<i32, i32> = BTreeMap::new();
        let mut rank_people_map: BTreeMap<i32, i32> = BTreeMap::new();
        let mut rank = 1;
        let mut now_rank = 1;
        let mut on_rank_people = 0;
        for users in &data.StandingsData {
            let mut is_rated = users.IsRated;
            if i.contest_type == 1 {
                is_rated = users.IsRated && users.TotalResult.Count > 0
            }
            if is_rated {
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
                if on_rank_people == 0 {
                    rank_people_map.insert(users.Rank, 1);
                } else {
                    rank_people_map.insert(users.Rank, on_rank_people);
                }
            }
        }

        let models = diff::get_diff(data.clone(), i.rating_range_end >= 0);

        println!("{:?}", models);
        let empty_set: BTreeSet<String> = BTreeSet::new();
        for (channel_id, server_id) in &channels {
            if channel_id != "null" {
                let mut last_rank = 0;
                let mut server_rank = 1;
                let mut rank_people = 0;

                let user_list = contest_users_map.get(server_id).unwrap_or(Clone::clone(&&empty_set));
                let mut ranks = vec![];
                let mut server_ranks = vec![];
                let mut users_list = vec![];
                let mut perf_list = vec![];
                let mut total = vec![];
                let mut old_rate_list = vec![];
                let mut new_rate_list = vec![];
                let mut rate_diff_list = vec![];
                let mut rated_list = vec![];

                let mut points = vec![];
                let mut task_name_to_index: BTreeMap<&str, usize> = BTreeMap::new();
                for (index, task) in data.TaskInfo.iter().enumerate() {
                    let mut difficulty = models[&task.Assignment].difficulty;
                    if difficulty <= 400.0 {
                        difficulty = 400.0 / (f64::exp((400.0 - difficulty) / 400.0))
                    }

                    points.push(TableRowsText {
                        title: Title::RatingCustom(RatingCustom {
                            title: task.Assignment.clone(),
                            color_theme: Theme::Dark,
                            rating: difficulty as i32,
                            has_bronze: false,
                        }),
                        width: 0,
                        align: Align::Middle,
                        data: vec![],
                    });
                    task_name_to_index.insert(&task.TaskScreenName, index);
                }

                let mut total_width = 0;
                let mut user_width = 0;
                for users in &data.StandingsData {
                    if user_list.contains(&users.UserScreenName.to_lowercase()) {
                        if last_rank != users.Rank {
                            server_rank += rank_people;
                            last_rank = users.Rank;
                            rank_people = 0;
                        }
                        let rank = (*rank_map.get(&users.Rank).unwrap() as f64) + ((rank_people_map.get(&users.Rank).unwrap() - 1) as f64) / 2.0;
                        let mut r = 6144.0;
                        let mut l = -2048.0;
                        while r - l > 0.5 {
                            let x = (r + l) / 2.0;
                            let mut sum = 0.5;
                            let contains_key = memo_data.contains_key(&Float::try_new(x).unwrap());
                            if !contains_key {
                                for j in &data.StandingsData {
                                    let mut is_rated = j.IsRated;
                                    if i.contest_type == 1 {
                                        is_rated = j.IsRated && j.TotalResult.Count > 0
                                    }
                                    if is_rated {
                                        let aperf = users_to_aperf.get(&j.UserScreenName.to_lowercase()).unwrap_or(match i.rating_type {
                                            2 => &(1200.0, 1000.0, 0, 0),
                                            1 => &(1000.0, 1000.0, 0, 0),
                                            _ => &(800.0, 1000.0, 0, 0),
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
                                                0 => 1000.0,
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
                            if rank > sum {
                                r = x;
                            } else {
                                l = x;
                            }
                        }
                        let mut perf = (r + l) / 2.0;
                        if perf <= 400.0 {
                            perf = 400.0 / (f64::exp((400.0 - perf) / 400.0))
                        }

                        if i.rating_range_end >= 0 && perf >= i.rating_range_end as f64 + 401.0 {
                            perf = i.rating_range_end as f64 + 401.0
                        }

                        let mut performance_list: Vec<i32> = conn
                            .exec(
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
                            )
                            .unwrap();
                        performance_list.push(perf as i32);

                        let mut rate = if i.contest_type == 0 {
                            performance_list.reverse();

                            let rated_contests = performance_list.len() as i32;

                            let numerator: f64 = (1..=rated_contests)
                                .map(|i| {
                                    let performance = performance_list[(i - 1) as usize];
                                    2.0_f64.powf(performance as f64 / 800.0) * 0.9_f64.powi(i)
                                })
                                .sum();

                            let denominator: f64 = (1..=rated_contests).map(|i| 0.9_f64.powi(i)).sum();

                            800.0 * (numerator / denominator).log2()
                                - ((f64::sqrt(1.0 - 0.81_f64.powi((performance_list.len() + 1) as i32))
                                    / (1.0 - 0.9_f64.powi((performance_list.len() + 1) as i32)))
                                    - 1.0)
                                    / (f64::sqrt(19.0) - 1.0)
                                    * 1200.0
                        } else {
                            let mut qs = vec![];
                            for i in performance_list {
                                for j in 1..=100 {
                                    qs.push(i as f64 - S * (j as f64).log(f64::consts::E));
                                }
                            }
                            qs.sort_by(|a, b| b.partial_cmp(a).unwrap());
                            let mut numerator: f64 = 0.0;
                            let mut denominator: f64 = 0.0;
                            for i in (0..=99).rev() {
                                numerator = numerator * R + qs[i];
                                denominator = denominator * R + 1.0;
                            }

                            numerator / denominator
                        };

                        if rate <= 400.0 {
                            rate = 400.0 / (f64::exp((400.0 - rate) / 400.0))
                        }

                        new_rate_list.push(RatingType::Custom(RatingCustom {
                            rating: rate as i32,
                            title: (rate as i32).to_string(),
                            has_bronze: false,
                            color_theme: Theme::Dark,
                        }));
                        old_rate_list.push(RatingType::Custom(RatingCustom {
                            rating: users.Rating,
                            title: users.Rating.to_string(),
                            has_bronze: false,
                            color_theme: Theme::Dark,
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
                            has_bronze: false,
                            color_theme: Theme::Dark,
                        }));
                        rated_list.push(TextConfig {
                            value: if users.IsRated { "Yes".to_string() } else { "No".to_string() },
                            color: if users.IsRated { "white" } else { "gray" }.to_string(),
                        });
                        let total_text = if users.TotalResult.Penalty > 0 {
                            total.push(TextConfig {
                                value: format!("{}<tspan fill=\"#f33\">({})</tspan>", users.TotalResult.Score / 100, users.TotalResult.Penalty),
                                color: "white".to_string(),
                            });
                            format!("{}({})", users.TotalResult.Score / 100, users.TotalResult.Penalty)
                        } else {
                            total.push(TextConfig {
                                value: format!("{}", users.TotalResult.Score / 100),
                                color: "white".to_string(),
                            });
                            format!("{}", users.TotalResult.Score / 100)
                        };
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&total_text, scale, 0));

                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32);
                        total_width = total_width.max(width as i32);

                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&users.UserScreenName, scale, 0));

                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32);
                        user_width = user_width.max(width as i32);
                        ranks.push(TextConfig {
                            value: ordinal_suffix(users.Rank),
                            color: match users.Rank {
                                1 => "#FFD700",
                                2 => "#C0C0C0",
                                3 => "#",
                                _ => "white",
                            }
                            .to_string(),
                        });
                        for (key, value) in &task_name_to_index {
                            let text = if !users.TaskResults.contains_key(*key) {
                                points[*value].data.push(TextConfig {
                                    value: "-".to_string(),
                                    color: "white".to_string(),
                                });
                                "-".to_string()
                            } else if users.TaskResults[*key].Penalty > 0 {
                                let task = &users.TaskResults[*key];
                                points[*value].data.push(TextConfig {
                                    value: format!("{}<tspan fill=\"#f33\">({})</tspan>", task.Score / 100, task.Penalty),
                                    color: "white".to_string(),
                                });
                                format!("{}({})", task.Score / 100, task.Penalty)
                            } else {
                                let task = &users.TaskResults[*key];
                                points[*value].data.push(TextConfig {
                                    value: format!("{}", task.Score / 100),
                                    color: "white".to_string(),
                                });
                                format!("{}", task.Score / 100)
                            };
                            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                            layout.append(&[font.clone()], &TextStyle::new(&text, scale, 0));
                            let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32) + 50.0;
                            points[*value].width = points[*value].width.max(width as i32);
                        }
                        server_ranks.push(TextConfig {
                            value: ordinal_suffix(server_rank),
                            color: match server_rank {
                                1 => "#FFD700",
                                2 => "#C0C0C0",
                                3 => "#CD7F32",
                                _ => "white",
                            }
                            .to_string(),
                        });
                        rank_people += 1;

                        users_list.push(RatingType::UserRating(create_table::UserRating {
                            username: users.UserScreenName.clone(),
                            contest_type: match i.contest_type {
                                0 => super::contest_type::ContestType::Algorithm,
                                _ => super::contest_type::ContestType::Heuristic,
                            },
                            color_theme: Theme::Dark,
                        }));
                    }
                }
                let rows = [
                    vec![
                        Row::Text(TableRowsText {
                            title: Title::Text("All".to_string()),
                            width: 300,
                            align: Align::Start,
                            data: ranks,
                        }),
                        Row::Text(TableRowsText {
                            title: Title::Text("Server".to_string()),
                            width: 300,
                            align: Align::Start,
                            data: server_ranks,
                        }),
                        Row::Rating(TableRowsRating {
                            title: Title::Text("User".to_string()),
                            width: user_width + 120,
                            data: users_list,
                        }),
                        Row::Text(TableRowsText {
                            title: Title::Text("Total".to_string()),
                            width: total_width + 50,
                            align: Align::Middle,
                            data: total,
                        }),
                    ],
                    points.iter().map(|x| Row::Text(x.clone())).collect(),
                    vec![
                        Row::Rating(TableRowsRating {
                            title: Title::Text("Perf".to_string()),
                            width: 300,
                            data: perf_list,
                        }),
                        Row::Rating(TableRowsRating {
                            title: Title::Text("Old".to_string()),
                            width: 300,
                            data: old_rate_list,
                        }),
                        Row::Rating(TableRowsRating {
                            title: Title::Text("New".to_string()),
                            width: 300,
                            data: new_rate_list,
                        }),
                        Row::Text(TableRowsText {
                            title: Title::Text("Diff".to_string()),
                            width: 300,
                            align: Align::Middle,
                            data: rate_diff_list,
                        }),
                        Row::Text(TableRowsText {
                            title: Title::Text("Rated".to_string()),
                            width: 300,
                            align: Align::Middle,
                            data: rated_list,
                        }),
                    ],
                ]
                .concat();
                let svg = create_table::create_table(&Arc::new(Mutex::new(pool_temp.clone())), format!("{} サーバー内ランキング", i.name), rows).await;
                let channel = ChannelId::new(channel_id.parse::<u64>().unwrap());
                let response = {
                    let mut message = CreateMessage::new();
                    message = message.add_file(CreateAttachment::bytes(
                        svg_to_png(svg.svg.as_str(), svg.width as u32, svg.height as u32, 1.0, 1.0),
                        "ranking.png",
                    ));
                    message
                };
                if contest_message_map.contains_key(&i.contest_id) {
                    let message_id: Vec<&Message> =
                        contest_message_map.get(&i.contest_id).unwrap().iter().filter(|x| x.channel_id.to_string() == channel_id.clone()).collect();
                    if !message_id.is_empty() {
                        let channel = ChannelId::new(message_id[0].channel_id as u64);
                        let _ = channel
                            .edit_message(
                                ctx.http.clone(),
                                message_id[0].message_id as u64,
                                EditMessage::default().new_attachment(CreateAttachment::bytes(
                                    svg_to_png(svg.svg.as_str(), svg.width as u32, svg.height as u32, 1.0, 1.0),
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
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    Ok(())
}
