use crate::{Context, Error};

#[derive(Debug, poise::ChoiceParameter)]
pub enum Languages {
    #[name = "ja"]
    A,
    #[name = "en"]
    B,
}

use mysql::prelude::*;
use mysql::*;

use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor};

/// Set the default language for the server.
#[poise::command(prefix_command, slash_command, rename = "set-language")]
pub async fn set_language(
    ctx: Context<'_>,
    #[description = "server default language"] language_code: Languages,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().to_string();

    let language_code_str: &str = match language_code {
        Languages::A => "ja",
        Languages::B => "en",
    };

    let count: Vec<i32> = conn.exec(
        r"SELECT count(*) FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;

    if count[0] == 0 {
        conn.exec_drop(
            r"INSERT INTO server_settings (server_id, language) VALUES (:server_id, :language)",
            params! {"server_id" => &guild_id, "language" => language_code_str},
        )?;
    } else {
        conn.exec_drop(
            r"UPDATE server_settings SET language=:language WHERE server_id=:server_id",
            params! {"server_id" => &guild_id, "language" => language_code_str},
        )?;
    }

    let response = {
        let mut embed = serenity::CreateEmbed::default().author(
            CreateEmbedAuthor::new("")
                .name("AtCoder Notify Bot v3")
                .icon_url(ctx.data().avater_url.as_str())
                .url("https://atcoder-notify.com/"),
        );
        if language_code_str == "ja" {
            embed = embed
                .title("設定変更")
                .description("デフォルトの言語設定を日本語に変更しました。");
        } else {
            embed = embed
                .title("Settings Changed")
                .description("The default language setting has been changed to English.");
        }
        poise::CreateReply::default().embed(embed).ephemeral(true)
    };

    ctx.send(response).await?;

    Ok(())
}
