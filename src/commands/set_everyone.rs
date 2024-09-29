use crate::{Context, Error};

use mysql::prelude::*;
use mysql::*;

use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedAuthor},
    CreateReply,
};

/// Set whether to notify everyone or not.
#[poise::command(prefix_command, slash_command, rename = "set-do-everyone")]
pub async fn set_everyone(ctx: Context<'_>, #[description = "do_everyone"] do_everyone: bool) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let do_everyone = if do_everyone { 1 } else { 0 };

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
            params! {"server_id" => &guild_id, "do_everyone" => do_everyone},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE server_settings SET do_everyone=:do_everyone WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "do_everyone" => do_everyone},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!("Everyone 通知を {} に変更しました。", if do_everyone == 0 { "オフ" } else { "オン" }));
        } else {
            embed = embed.title("Settings Changed").description(format!("Changed Everyone notification to {}.", if do_everyone == 0 { "off" } else { "on" }));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
