use crate::{Context, Error};

use mysql::prelude::*;
use mysql::*;

use poise::serenity_prelude::{self as serenity, json::NULL, CreateEmbedAuthor};

/// Set the channel for notifications about AtCoder contest information.
#[poise::command(prefix_command, slash_command, rename = "contest")]
pub async fn set_notification_contest(
    ctx: Context<'_>,
    #[description = "notify channel"] channel: serenity::Channel,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();
    let channel_id = channel.id().get();

    let count: Vec<i32> = conn.exec(
        r"SELECT count(*) FROM notifications WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    if count[0] == 0 {
        conn.exec_drop(
            r"INSERT INTO notifications (server_id, contest_channel_id) VALUES (:server_id, :channel)",
            params! {"server_id" => &guild_id, "channel" => channel_id},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE notifications SET contest_channel_id=:channel WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "channel" => channel_id},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default().author(
            CreateEmbedAuthor::new("")
                .name("AtCoder Notify Bot v3")
                .icon_url(ctx.data().avater_url.as_str())
                .url("https://atcoder-notify.com/"),
        );
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!(
                "コンテスト情報の通知チャンネルを <#{}> に設定しました。",
                channel_id
            ));
        } else {
            embed = embed.title("Settings Changed").description(format!(
                "Contest information notification channel set to <#{}>.",
                channel_id
            ));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}

/// Unset the channel for notifications about AtCoder contest information.
#[poise::command(prefix_command, slash_command, rename = "contest")]
pub async fn unset_notification_contest(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let count: Vec<i32> = conn.exec(
        r"SELECT count(*) FROM notifications WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    if count[0] == 0 {
        conn.exec_drop(
            r"INSERT INTO notifications (server_id, contest_channel_id) VALUES (:server_id, :channel)",
            params! {"server_id" => &guild_id, "channel" => NULL},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE notifications SET contest_channel_id=:channel WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "channel" => NULL},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default().author(
            CreateEmbedAuthor::new("")
                .name("AtCoder Notify Bot v3")
                .icon_url(ctx.data().avater_url.as_str())
                .url("https://atcoder-notify.com/"),
        );
        if lang == "ja" {
            embed = embed
                .title("設定変更")
                .description("コンテストの通知チャンネルを削除しました。");
        } else {
            embed = embed
                .title("Settings Changed")
                .description("Contest notification channels have been removed.");
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
