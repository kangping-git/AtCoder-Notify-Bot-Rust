use crate::{Context, Error};

#[derive(Debug, poise::ChoiceParameter)]
pub enum ACSettings {
    #[name = "all"]
    All,
    #[name = "unique"]
    Unique,
}

use mysql::prelude::*;
use mysql::*;

use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedAuthor},
    CreateReply,
};

/// Set the default language for the server.
#[poise::command(prefix_command, slash_command, rename = "set-notify-everyone")]
pub async fn set_notify_everyone(ctx: Context<'_>, #[description = "do everyone"] do_everyone: bool) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let selected_data: Vec<(String, i32)> = conn.exec(
        r"SELECT language,plan FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].0.as_str();
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

    let count: Vec<i32> = conn.exec(
        r"SELECT count(*) FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    if count[0] == 0 {
        conn.exec_drop(
            r"INSERT INTO server_settings (server_id, do_everyone) VALUES (:server_id, :do_everyone)",
            params! {"server_id" => &guild_id, "do_everyone" => do_everyone as i32},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE server_settings SET do_everyone=:do_everyone WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "do_everyone" => do_everyone as i32},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!(
                "1時間前通知でeveryoneを {} に変更しました。",
                if do_everyone { "する" } else { "しない" }
            ));
        } else {
            embed = embed.title("Settings Changed").description(format!(
                "Changed 1 hour notification to {} everyone.",
                if do_everyone { "do" } else { "not do" }
            ));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
