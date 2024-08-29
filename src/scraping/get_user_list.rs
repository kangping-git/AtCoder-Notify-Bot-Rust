use chrono::DateTime;
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{Context, GuildId, RoleId, UserId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub async fn user_list_update(conn: &Arc<Mutex<Pool>>, ctx: &Context) -> Result<()> {
    let pool = conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let start_time = Instant::now();
    let list: Vec<(i32, i32, i32, String, String, i8)> = conn.query("select * from user_ratings").unwrap();
    log::info!("get all of db {:?}", start_time.elapsed());
    let mut user_algo_history: BTreeMap<String, Vec<(i32, i32, String)>> = BTreeMap::new();
    let mut user_heuristic_history: BTreeMap<String, Vec<(i32, i32, String)>> = BTreeMap::new();
    let mut user_set = HashSet::new();
    for i in list {
        user_set.insert(i.4.clone().to_lowercase());
        let history = if i.5 == 0 {
            user_algo_history.entry(i.4.clone().to_lowercase()).or_default()
        } else {
            user_heuristic_history.entry(i.4.clone().to_lowercase()).or_default()
        };
        history.push((i.1, i.2, i.3));
    }

    let atcoder_users_vec: Vec<(u64, String, u64, i32)> = conn
        .query(
            r"SELECT
            users.discord_id,
            users.atcoder_username,
            users.server_id,
            COALESCE(atcoder_user_ratings.algo_rating, 0) AS algo_rating
        FROM
            users
        LEFT JOIN
            atcoder_user_ratings
        ON
            users.atcoder_username = atcoder_user_ratings.user_name AND users.discord_id IS NOT NULL
        WHERE
            users.discord_id IS NOT NULL",
        )
        .unwrap();
    log::info!("add to BTreeMap: {:?}", start_time.elapsed());
    let contests: Vec<(String, String)> = conn.query("select contest_id,start_time from contests").unwrap();
    let mut contest_data = BTreeMap::new();
    for (contest_id, start_time) in contests {
        contest_data.insert(contest_id, DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S%z").unwrap());
    }
    let mut user_rating_map = BTreeMap::new();
    let mut transaction = conn.start_transaction(TxOpts::default()).unwrap();
    transaction.query_drop("delete from atcoder_user_ratings").unwrap();
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
                let duration: chrono::Duration = *contest_data.get(&a.2).unwrap() - *contest_data.get(&b.2).unwrap();
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
                let duration: chrono::Duration = *contest_data.get(&a.2).unwrap() - *contest_data.get(&b.2).unwrap();
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
        user_rating_map.insert(i.clone().to_lowercase(), algo_rating);
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
    for i in atcoder_users_vec {
        let ur = user_rating_map.get(&i.1.to_lowercase()).unwrap_or(&0);
        let old_rating_color = if i.3 == 0 { 0 } else { std::cmp::min(8, i.3 / 400 + 1) };
        let new_rating_color = if ur == &0 { 0 } else { std::cmp::min(8, ur / 400 + 1) };
        if old_rating_color != new_rating_color {
            let roles: Vec<(u64, i8)> = transaction
                .exec(
                    "SELECT role_id, role_color FROM roles WHERE guild_id=:guild_id",
                    params! {
                        "guild_id" => i.2
                    },
                )
                .unwrap();
            if roles.is_empty() {
                continue;
            }
            let mut role_map = BTreeMap::new();
            for (role_id, role_color) in roles {
                role_map.insert(role_color, role_id);
            }
            let user = UserId::new(i.0);
            let member = GuildId::new(i.2).member(&ctx.http, user).await;
            if let Ok(member) = member {
                let _ = member.remove_role(&ctx.http, RoleId::new(*role_map.get(&(old_rating_color as i8)).unwrap_or(&0u64))).await;
                let _ = member.add_role(&ctx.http, RoleId::new(*role_map.get(&(new_rating_color as i8)).unwrap_or(&0u64))).await;
            }
        }
    }
    transaction.commit().unwrap();
    log::info!("add to Database: {:?}", start_time.elapsed());
    Ok(())
}
