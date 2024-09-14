use crate::{Context, Error};

use mysql::prelude::*;
use mysql::*;

use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedAuthor},
    CreateReply,
};

/// Set the channel for notifications about AtCoder user submission.
#[poise::command(prefix_command, slash_command, rename = "submission")]
pub async fn set_notification_submission(ctx: Context<'_>, #[description = "notify channel"] channel: serenity::Channel) -> Result<(), Error> {
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
    let owners: Vec<u64> = conn
        .exec(
            "SELECT user_id FROM owners WHERE guild_id=:guild_id",
            params! {
                "guild_id" => ctx.guild_id().unwrap_or_default().get()
            },
        )
        .unwrap();
    let has_permission = if owners.is_empty() || owners.contains(&{ ctx.author().id.get() }) {
        true
    } else {
        ctx.author().id.get() == ctx.guild().unwrap().owner_id.get()
    };
    if !has_permission {
        if lang == "ja" {
            let response = CreateReply::default().embed(CreateEmbed::default().title("エラー").description("権限がありません。")).ephemeral(true);
            ctx.send(response).await?;
        } else {
            let response = CreateReply::default().embed(CreateEmbed::default().title("Error").description("You do not have permission.")).ephemeral(true);
            ctx.send(response).await?;
        }
        return Ok(());
    }

    if count[0] == 0 {
        conn.exec_drop(
            r"INSERT INTO notifications (server_id, submission_channel_id) VALUES (:server_id, :channel)",
            params! {"server_id" => &guild_id, "channel" => channel_id},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE notifications SET submission_channel_id=:channel WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "channel" => channel_id},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!("ユーザーの提出情報の通知チャンネルを <#{}> に設定しました。", channel_id));
        } else {
            embed = embed.title("Settings Changed").description(format!("User submission information notification channel set to <#{}>.", channel_id));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}

/// Unet the channel for notifications about AtCoder user submission.
#[poise::command(prefix_command, slash_command, rename = "submission")]
pub async fn unset_notification_submission(ctx: Context<'_>) -> Result<(), Error> {
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

    if count[0] != 0 {
        conn.exec_drop(
            r"UPDATE notifications SET submission_channel_id=:channel WHERE server_id=NULL",
            params! {"server_id" => &guild_id},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description("ユーザーの提出情報の通知チャンネルを削除しました。");
        } else {
            embed = embed.title("Settings Changed").description("Removed notification channels for user submission information.");
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
