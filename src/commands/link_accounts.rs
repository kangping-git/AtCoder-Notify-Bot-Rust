use crate::{Context, Error};
use mysql::prelude::*;
use mysql::*;
use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor};

/// Link a specified AtCoder account with the current Discord account.
#[poise::command(prefix_command, slash_command, rename = "link-account")]
pub async fn link_account(
    ctx: Context<'_>,
    #[description = "discord_user"] discord_user: serenity::User,
    #[description = "atcoder_username"] atcoder_user: String,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string().parse::<i64>().unwrap();
    let link_accounts:Vec<i32> = conn.exec(
        "SELECT id FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
        params! {"discord_id" => discord_user.id.to_string().parse::<i64>().unwrap(),"server_id" => &guild_id},
    ).unwrap();
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

    let response = {
        let mut embed = serenity::CreateEmbed::default().author(
            CreateEmbedAuthor::new("")
                .name("AtCoder Notify Bot v3")
                .icon_url(ctx.data().avatar_url.as_str())
                .url("https://atcoder-notify.com/"),
        );
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!(
                "次のアカウントを連携しました。\n<@{}>と`{}`",
                discord_user.id, atcoder_user
            ));
        } else {
            embed = embed.title("Settings Changed").description(format!(
                "The following accounts have been linked. \n<@{}> and `{}`",
                discord_user.id, atcoder_user
            ));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}

/// Unlink the AtCoder account from the current Discord account.
#[poise::command(prefix_command, slash_command, rename = "unlink-account")]
pub async fn unlink_account(
    ctx: Context<'_>,
    #[description = "discord_user"] discord_user: serenity::User,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string().parse::<i64>().unwrap();
    let link_accounts:Vec<i32> = conn.exec(
        "SELECT id FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
        params! {"discord_id" => discord_user.id.to_string().parse::<i64>().unwrap(),"server_id" => &guild_id},
    ).unwrap();
    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    if !link_accounts.is_empty() {
        conn.exec_drop(
            r"delete from users where id=:id",
            params! {"id" => link_accounts[0]},
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
                    "<@{}>とのアカウントを連携を解除しましたしました。",
                    discord_user.id
                ));
            } else {
                embed = embed.title("Settings Changed").description(format!(
                    "We have de-linked your account with <@{}>.",
                    discord_user.id
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
                    "<@{}>とアカウントを連携されたアカウントがありません。",
                    discord_user.id
                ));
            } else {
                embed = embed.title("Error").description(format!(
                    "There is no account linked to <@{}>.",
                    discord_user.id
                ));
            }
            poise::CreateReply::default().embed(embed).ephemeral(true)
        };

        ctx.send(response).await?;
    }

    Ok(())
}

/// Show the AtCoder account currently linked to the Discord account.
#[poise::command(prefix_command, slash_command, rename = "show-linked-account")]
pub async fn show_linked_account(
    ctx: Context<'_>,
    #[description = "discord_user"] discord_user: Option<serenity::User>,
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

    match discord_user {
        Some(user) => {
            let user_id = user.id.to_string().parse::<i64>().unwrap();
            let atcoder_user: Vec<String> = conn.exec("SELECT atcoder_username FROM users WHERE discord_id=:discord_id AND server_id=:server_id AND discord_id IS NOT NULL", params! {
                "discord_id" => user_id,
                "server_id" => guild_id
            }).unwrap();
            let response = {
                let mut embed = serenity::CreateEmbed::default().author(
                    CreateEmbedAuthor::new("")
                        .name("AtCoder Notify Bot v3")
                        .icon_url(ctx.data().avatar_url.as_str())
                        .url("https://atcoder-notify.com/"),
                );
                if atcoder_user.is_empty() {
                    if lang == "ja" {
                        embed = embed.title("表示").description(format!(
                            "<@{}>と連携されたアカウントはありません。",
                            user_id
                        ));
                    } else {
                        embed = embed
                            .title("display")
                            .description(format!("There is no account linked to <@{}>.", user_id));
                    }
                } else if lang == "ja" {
                    embed = embed.title("表示").description(format!(
                        "<@{}>と連携されたアカウントは`{}`です。",
                        user_id, atcoder_user[0]
                    ));
                } else {
                    embed = embed.title("display").description(format!(
                        "The account linked to <@{}> is {}.",
                        user_id, atcoder_user[0]
                    ));
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
                    CreateEmbedAuthor::new("")
                        .name("AtCoder Notify Bot v3")
                        .icon_url(ctx.data().avatar_url.as_str())
                        .url("https://atcoder-notify.com/"),
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
