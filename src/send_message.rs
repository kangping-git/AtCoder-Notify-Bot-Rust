use std::sync::Arc;

use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{ChannelId, CreateEmbed, CreateMessage};
use tokio::sync::Mutex;

#[derive(Debug)]
struct Contest {
    start_time: String,
    duration: i32,
    rating_range_raw: String,
    name: String,
    contest_id: String,
}

pub async fn send_notify(pool: &Arc<Mutex<Pool>>, ctx: &serenity::Context) {
    let pool = pool.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let contests: Vec<Contest> = conn
        .query_map(
            "select contest_id,start_time,duration,rating_range_raw,name from contests",
            |(start_time, duration, rating_range_raw, name, contest_id)| Contest {
                start_time,
                duration,
                rating_range_raw,
                name,
                contest_id,
            },
        )
        .unwrap();
    let contests: Vec<&Contest> = contests
        .iter()
        .filter(|contest| {
            let start_time = chrono::DateTime::parse_from_str(&contest.start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
            let offset = chrono::Duration::minutes(contest.duration as i64);
            let end_time = start_time + offset;
            start_time.date_naive() <= chrono::Local::now().date_naive() && end_time.date_naive() >= chrono::Local::now().date_naive()
        })
        .collect();
    if !contests.is_empty() {
        let response = {
            let message = CreateMessage::new();
            let mut embed_vec = vec![];
            for contest in contests {
                let start_time = chrono::DateTime::parse_from_str(&contest.start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
                let offset = chrono::Duration::minutes(contest.duration as i64);
                let end_time = start_time + offset;
                let embed = CreateEmbed::new()
                    .title(&contest.name)
                    .url(contest.contest_id.clone())
                    .field("開催時間", format!("<t:{0}:f>(<t:{0}:R>)", start_time.timestamp()), false)
                    .field("終了時間", format!("<t:{0}:f>(<t:{0}:R>)", end_time.timestamp()), false)
                    .field("Rated対象", format!("`{}`", contest.rating_range_raw), false);
                embed_vec.push(embed);
            }
            message.content("今日のコンテストです").add_embeds(embed_vec)
        };
        let channels: Vec<String> = conn.query("SELECT contest_channel_id FROM notifications").unwrap();
        for i in channels {
            log::info!("{}", i);
            if i != "null" {
                let channel_id = i.parse::<u64>().unwrap();
                let channel = ChannelId::new(channel_id);
                let temp = channel.send_message(ctx.http.clone(), response.clone()).await;
                match temp {
                    Ok(t) => {
                        println!("{:?}", t);
                    }
                    Err(t) => {
                        println!("{:?}", t);
                    }
                }
            }
        }
    }
    conn.query_drop("delete from messages").unwrap();
}
