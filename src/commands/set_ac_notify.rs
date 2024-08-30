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
#[poise::command(prefix_command, slash_command, rename = "set-ac-notify")]
pub async fn set_ac_notify(ctx: Context<'_>, #[description = "ac_settings"] ac_settings: ACSettings) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let ac_type = match ac_settings {
        ACSettings::All => 0,
        ACSettings::Unique => 1,
    };

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
            r"INSERT INTO server_settings (server_id, ac_notify) VALUES (:server_id, :ac_notify)",
            params! {"server_id" => &guild_id, "ac_notify" => ac_type},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE server_settings SET ac_notify=:ac_notify WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "ac_notify" => ac_type},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default()
            .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
        if lang == "ja" {
            embed = embed.title("設定変更").description(format!(
                "通知するACを {} に変更しました。",
                if ac_type == 0 { "すべて" } else { "Unique AC のみ" }
            ));
        } else {
            embed = embed.title("Settings Changed").description(format!(
                "Changed the notification AC to {}.",
                if ac_type == 0 { "All" } else { "Unique AC only" }
            ));
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
