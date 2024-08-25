use std::sync::Arc;

use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, ChannelId, CreateEmbed, CreateMessage};
use tokio::sync::Mutex;

#[derive(Debug)]
struct Contest {
    start_time: String,
    duration: i32,
    rating_range_raw: String,
    name: String,
    contest_id: String,
}

pub async fn notify(pool: &Arc<Mutex<Pool>>, ctx: &serenity::Context) -> Result<()> {
    let pool = pool.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let contests: Vec<Contest> = conn
        .query_map(
            "select start_time,duration,rating_range_raw,name,contest_id from contests WHERE is_do_notify=0",
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
            let offset = chrono::Duration::hours(1);
            start_time - offset <= chrono::Local::now()
        })
        .collect();
    if !contests.is_empty() {
        let channels: Vec<(String, String)> = conn.query("SELECT server_id,contest_channel_id FROM notifications").unwrap();
        let (response_ja, response_en) = {
            let mut embed_vec_ja = vec![];
            let mut embed_vec_en = vec![];
            for contest in contests {
                let start_time = chrono::DateTime::parse_from_str(&contest.start_time, "%Y-%m-%d %H:%M:%S%z").unwrap();
                let offset = chrono::Duration::minutes(contest.duration as i64);
                let end_time = start_time + offset;
                let embed_ja = CreateEmbed::new()
                    .title(format!("{}が一時間後に開催されます", contest.name))
                    .url(format!("https://{}", &contest.contest_id))
                    .field("開催時間", format!("<t:{0}:f>(<t:{0}:R>)", start_time.timestamp()), false)
                    .field("終了時間", format!("<t:{0}:f>(<t:{0}:R>)", end_time.timestamp()), false)
                    .field("Rated対象", format!("`{}`", contest.rating_range_raw), false);
                let embed_en = CreateEmbed::new()
                    .title(format!("{} will be held in an hour", contest.name))
                    .url(format!("https://{}", &contest.contest_id))
                    .field("Start time", format!("<t:{0}:f>(<t:{0}:R>)", start_time.timestamp()), false)
                    .field("End time", format!("<t:{0}:f>(<t:{0}:R>)", end_time.timestamp()), false)
                    .field("Rated target", format!("`{}`", contest.rating_range_raw), false);
                embed_vec_ja.push(embed_ja);
                embed_vec_en.push(embed_en);
                conn.exec_drop(
                    "UPDATE contests SET is_do_notify=1 WHERE contest_id=:contest_id",
                    params! {"contest_id" => &contest.contest_id},
                )
                .unwrap();
            }
            (
                CreateMessage::new().content("@everyone").add_embeds(embed_vec_ja),
                CreateMessage::new().content("@everyone").add_embeds(embed_vec_en),
            )
        };
        for i in channels {
            if i.1 != "null" {
                let selected_data: Vec<String> = conn
                    .exec(
                        r"SELECT language FROM server_settings WHERE server_id=:server_id",
                        params! {"server_id" => i.0.parse::<u64>().unwrap()},
                    )
                    .unwrap();
                let mut lang = "ja";
                if selected_data.len() == 1 {
                    lang = selected_data[0].as_str();
                }

                let channel_id = ChannelId::new(i.1.parse::<u64>().unwrap());
                if lang == "ja" {
                    let _ = channel_id.send_message(&ctx.http, response_ja.clone()).await;
                } else {
                    let _ = channel_id.send_message(&ctx.http, response_en.clone()).await;
                }
            }
        }
    }
    Ok(())
}
