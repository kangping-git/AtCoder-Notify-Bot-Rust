use crate::{Context, Error};

use mysql::prelude::*;
use mysql::*;

use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor};

/// Display the currently set notification channels for contests and submissions.
#[poise::command(prefix_command, slash_command, rename = "show-notification")]
pub async fn show_notification(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let settings: Vec<(String, String)> = conn.exec(
        r"SELECT contest_channel_id,submission_channel_id FROM notifications WHERE server_id=:server_id",
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

    let mut contest_channel_id = "null".to_string();
    let mut submission_channel_id = "null".to_string();
    if !settings.is_empty() {
        contest_channel_id.clone_from(&settings[0].0);
        submission_channel_id.clone_from(&settings[0].1);
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("現在のサーバー設定");
            if contest_channel_id == "null" {
                embed = embed.field("コンテスト情報", "未設定", false);
            } else {
                embed = embed.field("コンテスト情報", format!("<#{}>", contest_channel_id), false);
            }
            if submission_channel_id == "null" {
                embed = embed.field("ユーザー提出情報", "未設定", false);
            } else {
                embed = embed.field("ユーザー提出情報", format!("<#{}>", submission_channel_id), false);
            }
        } else {
            embed = embed.title("Current Server Settings");
            if contest_channel_id == "null" {
                embed = embed.field("Contest Information", "Not Set", false);
            } else {
                embed = embed.field("Contest Information", format!("<#{}>", contest_channel_id), false);
            }
            if submission_channel_id == "null" {
                embed = embed.field("User Submission Information", "Not Set", false);
            } else {
                embed = embed.field("User Submission Information", format!("<#{}>", submission_channel_id), false);
            }
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
