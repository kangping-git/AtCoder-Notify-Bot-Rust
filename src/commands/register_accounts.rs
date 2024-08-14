use crate::{Context, Error};
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor};

/// Register a specified AtCoder account without linking it to the Discord account.
#[poise::command(prefix_command, slash_command, rename = "register-account")]
pub async fn register_account(
    ctx: Context<'_>,
    #[description = "atcoder_username"] atcoder_user: String,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string().parse::<i64>().unwrap();
    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    let link_accounts:Vec<i32> = conn.exec(
        "SELECT id FROM users WHERE atcoder_username=:atcoder_username AND server_id=:server_id AND discord_id IS NULL",
        params! {"atcoder_username" => &atcoder_user,"server_id" => &guild_id},
    ).unwrap();
    if link_accounts.is_empty() {
        conn.exec_drop(
            r"INSERT INTO users (server_id, atcoder_username) VALUES (:server_id, :atcoder_username)",
            params! {"server_id" => &guild_id, "atcoder_username" => &atcoder_user},
        )?;

        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed.title("設定変更").description(format!(
                    "次のアカウントを追加しました。\n`{}`",
                    atcoder_user
                ));
            } else {
                embed = embed.title("Settings Changed").description(format!(
                    "The following account was added\n`{}`",
                    atcoder_user
                ));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    } else {
        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed.title("エラー").description("すでに追加されています");
            } else {
                embed = embed
                    .title("Error")
                    .description("It has already been added.");
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    }
    Ok(())
}

/// Delete the registration of a specified AtCoder account.
#[poise::command(prefix_command, slash_command, rename = "delete-account")]
pub async fn delete_account(
    ctx: Context<'_>,
    #[description = "atcoder_user"] atcoder_user: String,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string().parse::<i64>().unwrap();
    let registered_accounts:Vec<i32> = conn.exec(
        "SELECT id FROM users WHERE atcoder_username=:atcoder_username AND server_id=:server_id AND discord_id IS NULL",
        params! {"atcoder_username" => &atcoder_user,"server_id" => &guild_id},
    ).unwrap();
    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    if !registered_accounts.is_empty() {
        conn.exec_drop(
            r"delete from users where id=:id",
            params! {"id" => registered_accounts[0]},
        )?;
        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed.title("設定変更").description(format!(
                    "`{}`のアカウントを連携を削除しました。",
                    &atcoder_user
                ));
            } else {
                embed = embed.title("Settings Changed").description(format!(
                    "I have removed the account linked to `{}`.",
                    &atcoder_user
                ));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    } else {
        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed.title("エラー").description(format!(
                    "{}というアカウントはまだ連携されていません。",
                    &atcoder_user
                ));
            } else {
                embed = embed.title("Error").description(format!(
                    "The account `{}` is not yet linked.",
                    &atcoder_user
                ));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    }

    Ok(())
}

/// Display a list of all currently registered AtCoder accounts.
#[poise::command(prefix_command, slash_command, rename = "show-account")]
pub async fn show_accounts(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string().parse::<i64>().unwrap();

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    let selected_data: Vec<String> = conn.exec(
        r"SELECT atcoder_username FROM users WHERE server_id=:server_id AND discord_id IS NULL",
        params! {"server_id" => &guild_id},
    )?;

    if selected_data.is_empty() {
        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed
                    .title("エラー")
                    .description("連携されたアカウントがありません");
            } else {
                embed = embed
                    .title("Error")
                    .description("You have no linked accounts.");
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    } else {
        let response = {
            let mut embed = serenity::CreateEmbed::default().author(
                CreateEmbedAuthor::new("")
                    .name("AtCoder Notify Bot v3")
                    .icon_url(ctx.data().avatar_url.as_str())
                    .url("https://atcoder-notify.com/"),
            );
            if lang == "ja" {
                embed = embed.title("表示").description(format!(
                    "連携されたアカウントは以下の通りです\n`{}`",
                    selected_data.join("`\n`")
                ));
            } else {
                embed = embed.title("Error").description(format!(
                    "The linked accounts are as follows:\n`{}`",
                    selected_data.join("`\n`")
                ));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    }

    Ok(())
}
