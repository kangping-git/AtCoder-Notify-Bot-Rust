use std::collections::{BTreeMap, BTreeSet};

use crate::{scraping::get_submission::Submission, Context, Error};
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor, RoleId, UserId};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserSubmissionCount {
    count: i32,
}

/// Link a specified AtCoder account with the current Discord account.
#[poise::command(prefix_command, slash_command, rename = "link-account")]
pub async fn link_account(
    ctx: Context<'_>,
    #[description = "discord_user"] discord_user: serenity::User,
    #[description = "atcoder_username"] atcoder_user: String,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();
    let link_accounts: Vec<i32> = conn
        .exec(
            "SELECT id FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
            params! {"discord_id" => discord_user.id.to_string().parse::<i64>().unwrap(),"server_id" => &guild_id},
        )
        .unwrap();
    ctx.defer_ephemeral().await?;
    if link_accounts.is_empty() {
        conn.exec_drop(
            r"INSERT INTO users (server_id, discord_id, atcoder_username) VALUES (:server_id, :discord_id, :atcoder_username)",
            params! {"server_id" => &guild_id, "discord_id" => discord_user.id.to_string().parse::<i64>().unwrap(), "atcoder_username" => &atcoder_user},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE users SET atcoder_username=:atcoder_username WHERE id=:id",
            params! {"id" => &link_accounts[0], "atcoder_username" => &atcoder_user},
        )?;
    }
    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    let roles: Vec<(i8, u64)> = conn.exec(
        r"SELECT role_color,role_id FROM roles WHERE guild_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    if !roles.is_empty() {
        let roles_map: BTreeMap<i8, u64> = roles.iter().cloned().collect();

        let guild = ctx.guild_id().unwrap();
        let role_ids: Vec<u64> = roles.iter().map(|x| x.1).collect();
        let user = UserId::new(discord_user.id.get());
        let member = guild.member(ctx.http(), user).await?;
        for i in &member.roles {
            if role_ids.contains(&i.get()) {
                let user = UserId::new(discord_user.id.get());
                let member = guild.member(ctx.http(), user).await?;
                member.remove_role(ctx.http(), *i).await?;
            }
        }

        let ratings: Vec<i64> = conn
            .exec(
                "SELECT algo_rating FROM atcoder_user_ratings WHERE user_name=:user_name",
                params! {
                    "user_name" => &atcoder_user
                },
            )
            .unwrap();
        let rating = if ratings.is_empty() { 0 } else { ratings[0] };
        let user = UserId::new(discord_user.id.get());
        let member = guild.member(ctx.http(), user).await?;
        if rating == 0 {
            member.add_role(ctx.http(), RoleId::new(*roles_map.get(&0).unwrap_or(&0))).await?;
        } else {
            member
                .add_role(
                    ctx.http(),
                    RoleId::new(*roles_map.get(&(std::cmp::min(8, rating / 400 + 1) as i8)).unwrap_or(&0)),
                )
                .await?;
        }
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!("次のアカウントを連携しました。\n<@{}>と`{}`", discord_user.id, atcoder_user));
        } else {
            embed = embed.title("Settings Changed").description(format!(
                "The following accounts have been linked. \n<@{}> and `{}`",
                discord_user.id, atcoder_user
            ));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    let user_submission: Vec<u64> = conn.exec(
        r"SELECT epoch_second FROM submissions WHERE username=:username",
        params! {"username" => &atcoder_user},
    )?;
    if user_submission.is_empty() {
        let submission_count_url = format!(
            "https://kenkoooo.com/atcoder/atcoder-api/v3/user/submission_count?user={}&from_second=0&to_second={}",
            atcoder_user,
            chrono::Utc::now().timestamp()
        );
        let submission_count_text = reqwest::get(submission_count_url).await?.text().await?;
        let submission_count: UserSubmissionCount = serde_json::from_str(&submission_count_text)?;
        let submissions_per_page = 500;
        let mut last_epoch = 0;
        let mut submission_set = BTreeSet::new();
        for _ in 0..((submission_count.count / submissions_per_page) + 1) {
            let submission_url = format!(
                "https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={}",
                atcoder_user, last_epoch
            );
            let submission_text = reqwest::get(submission_url).await?.text().await?;
            let submission_json: Vec<Submission> = serde_json::from_str(&submission_text)?;
            for submission in submission_json {
                submission_set.insert(submission.problem_id);
                last_epoch = submission.epoch_second.max(last_epoch);
            }
            last_epoch += 1;
        }
        conn.exec_batch(
            r"INSERT INTO submission_data (username, problem_id) VALUES (:username, :problem_id)",
            submission_set.iter().map(|problem_id| {
                params! {
                    "user_id" => &atcoder_user,
                    "problem_id" => problem_id,
                }
            }),
        )?;
        conn.exec_drop(
            r"INSERT INTO submissions (username, epoch_second) VALUES (:username, :epoch_second)",
            params! {"username" => &atcoder_user, "epoch_second" => last_epoch},
        )?;
    }

    Ok(())
}

/// Unlink the AtCoder account from the current Discord account.
#[poise::command(prefix_command, slash_command, rename = "unlink-account")]
pub async fn unlink_account(ctx: Context<'_>, #[description = "discord_user"] discord_user: serenity::User) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();
    let link_accounts: Vec<i32> = conn
        .exec(
            "SELECT id FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
            params! {"discord_id" => discord_user.id.to_string().parse::<i64>().unwrap(),"server_id" => &guild_id},
        )
        .unwrap();
    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    ctx.defer_ephemeral().await?;

    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    let roles: Vec<(i8, u64)> = conn.exec(
        r"SELECT role_color,role_id FROM roles WHERE guild_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let guild = ctx.guild_id().unwrap();
    let role_ids: Vec<u64> = roles.iter().map(|x| x.1).collect();
    let user = UserId::new(discord_user.id.get());
    let member = guild.member(ctx.http(), user).await?;
    for i in &member.roles {
        if role_ids.contains(&i.get()) {
            let user = UserId::new(discord_user.id.get());
            let member = guild.member(ctx.http(), user).await?;
            member.remove_role(ctx.http(), *i).await?;
        }
    }

    if !link_accounts.is_empty() {
        conn.exec_drop(r"delete from users where id=:id", params! {"id" => link_accounts[0]})?;
        let response = {
            let mut embed = serenity::CreateEmbed::default()
                .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
            if lang == "ja" {
                embed = embed.title("設定変更").description(format!("<@{}>とのアカウントを連携を解除しましたしました。", discord_user.id));
            } else {
                embed = embed.title("Settings Changed").description(format!("We have de-linked your account with <@{}>.", discord_user.id));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    } else {
        let response = {
            let mut embed = serenity::CreateEmbed::default()
                .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
            if lang == "ja" {
                embed = embed.title("エラー").description(format!("<@{}>とアカウントを連携されたアカウントがありません。", discord_user.id));
            } else {
                embed = embed.title("Error").description(format!("There is no account linked to <@{}>.", discord_user.id));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    }

    Ok(())
}

/// Show the AtCoder account currently linked to the Discord account.
#[poise::command(prefix_command, slash_command, rename = "show-linked-account")]
pub async fn show_linked_account(ctx: Context<'_>, #[description = "discord_user"] discord_user: Option<serenity::User>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    match discord_user {
        Some(user) => {
            let user_id = user.id.to_string().parse::<i64>().unwrap();
            let atcoder_user: Vec<String> = conn
                .exec(
                    "SELECT atcoder_username FROM users WHERE discord_id=:discord_id AND server_id=:server_id AND discord_id IS NOT NULL",
                    params! {
                        "discord_id" => user_id,
                        "server_id" => guild_id
                    },
                )
                .unwrap();
            let response = {
                let mut embed = serenity::CreateEmbed::default().author(
                    CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"),
                );
                if atcoder_user.is_empty() {
                    if lang == "ja" {
                        embed = embed.title("表示").description(format!("<@{}>と連携されたアカウントはありません。", user_id));
                    } else {
                        embed = embed.title("display").description(format!("There is no account linked to <@{}>.", user_id));
                    }
                } else if lang == "ja" {
                    embed = embed.title("表示").description(format!("<@{}>と連携されたアカウントは`{}`です。", user_id, atcoder_user[0]));
                } else {
                    embed = embed.title("display").description(format!("The account linked to <@{}> is {}.", user_id, atcoder_user[0]));
                }
                poise::CreateReply::default().embed(embed).ephemeral(true)
            };

            ctx.send(response).await?;
        }
        None => {
            let atcoder_users: Vec<(i64, String)> = conn
                .exec(
                    "SELECT discord_id,atcoder_username FROM users WHERE server_id=:server_id AND discord_id IS NOT NULL",
                    params! {
                        "server_id" => guild_id
                    },
                )
                .unwrap();
            let response = {
                let mut embed = serenity::CreateEmbed::default().author(
                    CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"),
                );
                let mut list: Vec<String> = vec![];
                for i in atcoder_users {
                    list.push(format!("<@{}> => `{}`", i.0, i.1))
                }
                if lang == "ja" {
                    embed = embed.title("表示").description(list.join("\n"));
                } else {
                    embed = embed.title("display").description(list.join("\n"));
                }
                poise::CreateReply::default().embed(embed).ephemeral(true)
            };
            ctx.send(response).await?;
        }
    }

    Ok(())
}
