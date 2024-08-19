use crate::scraping::contest_type::{ContestRatingType, ContestType};
use crate::utils::svg::create_table::{create_table, Align, Row, TableRowsText, TextConfig};
use crate::utils::svg_to_png::svg_to_png;
use crate::{Context, Error};
use chrono::{DateTime, FixedOffset};
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{CreateActionRow, CreateAttachment, CreateButton};
use poise::CreateReply;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Contest {
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub contest_type: ContestType,
    pub rating_type: ContestRatingType,
    pub name: String,
    pub rating_raw: String,
}

pub async fn create_contest_response(
    title: &str,
    pool: Pool,
    contests: Vec<&Contest>,
    components: Vec<CreateActionRow>,
    start_no: i32,
) -> (Vec<CreateActionRow>, CreateAttachment) {
    let mut counts: Vec<TextConfig> = vec![];
    let mut contest_type: Vec<TextConfig> = vec![];
    let mut contest_rating_type: Vec<TextConfig> = vec![];
    let mut contest_names: Vec<TextConfig> = vec![];
    let mut rating_range: Vec<TextConfig> = vec![];
    if contests.is_empty() {
        contest_names.push(TextConfig {
            value: "(empty)".to_string(),
            color: "white".to_string(),
        })
    }
    let mut c = start_no;
    for contest in contests {
        c += 1;
        counts.push(TextConfig {
            value: ordinal_suffix(c),
            color: "white".to_string(),
        });
        contest_type.push(TextConfig {
            value: match contest.contest_type {
                ContestType::Algorithm => "Algorithm",
                ContestType::Heuristic => "Heuristic",
            }
            .to_string(),
            color: "white".to_string(),
        });
        contest_names.push(TextConfig {
            value: contest.name.to_string(),
            color: "white".to_string(),
        });
        rating_range.push(TextConfig {
            value: contest.rating_raw.to_string(),
            color: "white".to_string(),
        });
        contest_rating_type.push(TextConfig {
            value: "◉".to_string(),
            color: match contest.rating_type {
                ContestRatingType::ABC => "blue",
                ContestRatingType::ARC => "orange",
                ContestRatingType::AGC => "red",
                ContestRatingType::None => "white",
            }
            .to_string(),
        });
    }
    let rows = vec![
        Row::Text(TableRowsText {
            title: "No.".to_string(),
            width: 200,
            align: Align::End,
            data: counts,
        }),
        Row::Text(TableRowsText {
            title: "".to_string(),
            width: 100,
            align: Align::Middle,
            data: contest_rating_type,
        }),
        Row::Text(TableRowsText {
            title: "ContestName".to_string(),
            width: 3500,
            align: Align::Start,
            data: contest_names,
        }),
        Row::Text(TableRowsText {
            title: "RatingType".to_string(),
            width: 500,
            align: Align::Middle,
            data: contest_type,
        }),
        Row::Text(TableRowsText {
            title: "RatingRange".to_string(),
            width: 700,
            align: Align::Middle,
            data: rating_range,
        }),
    ];
    let file = create_table(&Arc::new(Mutex::new(pool.clone())), title.to_string(), rows).await;
    let attachment = CreateAttachment::bytes(
        svg_to_png(
            file.svg.as_str(),
            file.width as u32 / 4,
            file.height as u32 / 4,
            0.25,
            0.25,
        ),
        "contests.png",
    );
    (components, attachment)
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

#[poise::command(
    prefix_command,
    slash_command,
    subcommands("past", "upcoming", "current")
)]
pub async fn contest(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Display information about upcoming AtCoder contests.
#[poise::command(prefix_command, slash_command)]
#[allow(clippy::needless_range_loop)]
async fn upcoming(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();

    let contests: Vec<Contest> = conn
        .query_map(
            "select start_time,duration,contest_type,rating_type,name,rating_range_raw from contests",
            |( start_time, duration, contest_type, rating_type, name,rating_raw): (
                String,
                i64,
                i8,
                i8,
                String,String
            )| {
                let start_time =
                    chrono::DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
                let offset = chrono::Duration::minutes(duration);
                Contest {
                    start_time,
                    end_time: start_time + offset,
                    contest_type: match contest_type {
                        0 => ContestType::Algorithm,
                        _ => ContestType::Heuristic,
                    },
                    rating_type: match rating_type {
                        0 => ContestRatingType::ABC,
                        1 => ContestRatingType::ARC,
                        2 => ContestRatingType::AGC,
                        _ => ContestRatingType::None,
                    },
                    name,rating_raw
                }
            },
        )
        .unwrap();
    let mut contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| chrono::Local::now() <= contest.start_time)
        .collect();
    contests.sort_by(|a, b| a.end_time.partial_cmp(&(b.end_time)).unwrap());

    let (components, attachment) =
        create_contest_response("upcoming contests", pool.clone(), contests, vec![], 0).await;

    let reply = CreateReply::default()
        .components(components)
        .attachment(attachment)
        .ephemeral(true);
    ctx.send(reply).await?;

    Ok(())
}

/// Show details about contests that are currently ongoing.
#[poise::command(prefix_command, slash_command)]
#[allow(clippy::needless_range_loop)]
async fn current(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();

    let contests: Vec<Contest> = conn
        .query_map(
            "select start_time,duration,contest_type,rating_type,name,rating_range_raw from contests",
            |( start_time, duration, contest_type, rating_type, name,rating_raw): (
                String,
                i64,
                i8,
                i8,
                String,String
            )| {
                let start_time =
                    chrono::DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
                let offset = chrono::Duration::minutes(duration);
                Contest {
                    start_time,
                    end_time: start_time + offset,
                    contest_type: match contest_type {
                        0 => ContestType::Algorithm,
                        _ => ContestType::Heuristic,
                    },
                    rating_type: match rating_type {
                        0 => ContestRatingType::ABC,
                        1 => ContestRatingType::ARC,
                        2 => ContestRatingType::AGC,
                        _ => ContestRatingType::None,
                    },
                    name,rating_raw
                }
            },
        )
        .unwrap();
    let mut contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| {
            contest.start_time <= chrono::Local::now() && chrono::Local::now() <= contest.end_time
        })
        .collect();
    contests.sort_by(|a, b| a.end_time.partial_cmp(&(b.end_time)).unwrap());

    let (components, attachment) =
        create_contest_response("current contests", pool.clone(), contests, vec![], 0).await;

    let reply = CreateReply::default()
        .components(components)
        .attachment(attachment)
        .ephemeral(true);

    ctx.send(reply).await?;

    Ok(())
}

/// Retrieve information on past AtCoder contests.
#[poise::command(prefix_command, slash_command)]
#[allow(clippy::needless_range_loop)]
async fn past(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();

    let contests: Vec<Contest> = conn
        .query_map(
            "select start_time,duration,contest_type,rating_type,name,rating_range_raw from contests",
            |(start_time, duration, contest_type, rating_type, name,rating_raw): (

                String,
                i64,
                i8,
                i8,
                String,String
            )| {
                let start_time =
                    chrono::DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
                let offset = chrono::Duration::minutes(duration);
                Contest {
                    start_time,
                    end_time: start_time + offset,
                    contest_type: match contest_type {
                        0 => ContestType::Algorithm,
                        _ => ContestType::Heuristic,
                    },
                    rating_type: match rating_type {
                        0 => ContestRatingType::ABC,
                        1 => ContestRatingType::ARC,
                        2 => ContestRatingType::AGC,
                        _ => ContestRatingType::None,
                    },
                    name,rating_raw
                }
            },
        )
        .unwrap();
    let mut contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| chrono::Local::now() >= contest.start_time)
        .collect();
    contests.sort_by(|a, b| b.end_time.partial_cmp(&(a.end_time)).unwrap());

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("goto_0")
            .label("<")
            .style(poise::serenity_prelude::ButtonStyle::Primary)
            .disabled(true),
        CreateButton::new("goto_1")
            .label(">")
            .style(poise::serenity_prelude::ButtonStyle::Primary),
    ])];

    let (components, attachment) = create_contest_response(
        "past contests (page 1)",
        pool.clone(),
        contests[0..20].to_vec(),
        components,
        0,
    )
    .await;

    let reply = CreateReply::default()
        .components(components)
        .attachment(attachment)
        .ephemeral(true);

    ctx.send(reply).await?;

    Ok(())
}
