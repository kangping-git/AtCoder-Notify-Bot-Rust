use crate::{Context, Error};
use serde::{Deserialize, Serialize};

use mysql::prelude::*;
use mysql::*;

use poise::serenity_prelude::{self as serenity, CreateEmbedAuthor};

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct CommandDesciption {
    name: String,
    usage: String,
    description: Vec<String>,
    is_owner_only: bool,
}

/// Display a list of all available commands and their usage.
#[poise::command(prefix_command, slash_command)]
pub async fn help(ctx: Context<'_>, #[description = "only_admin"] only_admin: Option<bool>) -> Result<(), Error> {
    let only_admin = only_admin.unwrap_or(false);

    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();

    let guild_id = ctx.guild_id().unwrap().to_string();

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => guild_id},
    )?;

    let mut lang = "ja";

    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    let help_obj: Vec<CommandDesciption> = if lang == "ja" {
        serde_json::from_str(include_str!("../assets/commands_ja.json")).unwrap()
    } else {
        serde_json::from_str(include_str!("../assets/commands_en.json")).unwrap()
    };

    let mut embed = serenity::CreateEmbed::default()
        .title(if only_admin { "Owner Only Commands" } else { "Commands" })
        .author(CreateEmbedAuthor::new("").name("AtCoder Notify Bot v3").icon_url(ctx.data().avatar_url.as_str()).url("https://atcoder-notify.com/"));
    for i in help_obj {
        if only_admin {
            if i.is_owner_only {
                embed = embed.field(i.usage, i.description.join("\n"), false);
            }
        } else {
            embed = embed.field(
                if i.is_owner_only {
                    format!("[**Owner Only**] {}", i.usage)
                } else {
                    i.usage.to_string()
                },
                i.description.join("\n"),
                false,
            );
        }
    }

    let response = poise::CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(response).await?;

    Ok(())
}
