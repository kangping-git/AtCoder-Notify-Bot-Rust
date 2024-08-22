use mysql::prelude::Queryable;
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use poise::CreateReply;

use crate::{Context, Error};
use mysql::*;

#[allow(dead_code)]
static PLAN_OWNER_NUMBERS: [usize; 4] = [1, 20, 998244353, 1];

#[poise::command(prefix_command, slash_command, subcommands("owner_add", "owner_remove", "show_owners"))]
pub async fn owner(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, rename = "add")]
pub async fn owner_add(ctx: Context<'_>, #[description = "add_user"] add_user: serenity::User) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();

    let selected_data: Vec<(String, i32)> = conn.exec(
        r"SELECT language,plan FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => &guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].0.as_str();
    }

    let plan = if selected_data.is_empty() { 0 } else { selected_data[0].1 };

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

    let max_owner_number = PLAN_OWNER_NUMBERS[plan as usize];
    if owners.len() >= max_owner_number {
        if lang == "ja" {
            let response =
                CreateReply::default().embed(CreateEmbed::default().title("エラー").description("オーナーの数が上限に達しています。")).ephemeral(true);
            ctx.send(response).await?;
        } else {
            let response =
                CreateReply::default().embed(CreateEmbed::default().title("Error").description("The number of owners has reached the limit.")).ephemeral(true);
            ctx.send(response).await?;
        }
        return Ok(());
    }

    if owners.contains(&add_user.id.get()) {
        if lang == "ja" {
            let response = CreateReply::default().embed(CreateEmbed::default().title("エラー").description("そのユーザーは既にオーナーです。")).ephemeral(true);
            ctx.send(response).await?;
        } else {
            let response = CreateReply::default().embed(CreateEmbed::default().title("Error").description("That user is already an owner.")).ephemeral(true);
            ctx.send(response).await?;
        }
        return Ok(());
    }

    conn.exec_drop(
        "INSERT INTO owners (guild_id, user_id) VALUES (:guild_id, :user_id)",
        params! {
            "guild_id" => ctx.guild_id().unwrap_or_default().get(),
            "user_id" => add_user.id.get()
        },
    )?;

    if lang == "ja" {
        let response =
            CreateReply::default().embed(CreateEmbed::default().title("オーナー追加").description(format!("オーナーに<@{}>を追加しました。", add_user.id)));
        ctx.send(response).await?;
    } else {
        let response = CreateReply::default().embed(CreateEmbed::default().title("Owner Added").description(format!("Added <@{}> to the owner.", add_user.id)));
        ctx.send(response).await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, rename = "remove")]
async fn owner_remove(ctx: Context<'_>, #[description = "remove_user"] remove_user: serenity::User) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();

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

    if !owners.contains(&remove_user.id.get()) {
        if lang == "ja" {
            let response =
                CreateReply::default().embed(CreateEmbed::default().title("エラー").description("そのユーザーはオーナーではありません。")).ephemeral(true);
            ctx.send(response).await?;
        } else {
            let response = CreateReply::default().embed(CreateEmbed::default().title("Error").description("That user is not an owner.")).ephemeral(true);
            ctx.send(response).await?;
        }
        return Ok(());
    }

    conn.exec_drop(
        "DELETE FROM owners WHERE guild_id=:guild_id AND user_id=:user_id",
        params! {
            "guild_id" => ctx.guild_id().unwrap_or_default().get(),
            "user_id" => remove_user.id.get()
        },
    )?;

    if lang == "ja" {
        let response = CreateReply::default()
            .embed(CreateEmbed::default().title("オーナー削除").description(format!("オーナーから<@{}>を削除しました。", remove_user.id)));
        ctx.send(response).await?;
    } else {
        let response =
            CreateReply::default().embed(CreateEmbed::default().title("Owner Removed").description(format!("Removed <@{}> from the owner.", remove_user.id)));
        ctx.send(response).await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
async fn show_owners(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();

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

    if lang == "ja" {
        let mut response = CreateReply::default();
        let owners_text = owners.iter().fold(String::new(), |acc, owner| acc + &format!("<@{}>\n", owner));
        response = response.embed(CreateEmbed::new().title("オーナー一覧").description(owners_text)).ephemeral(true);
        ctx.send(response).await?;
    } else {
        let mut response = CreateReply::default();
        let owners_text = owners.iter().fold(String::new(), |acc, owner| acc + &format!("<@{}>\n", owner));
        response = response.embed(CreateEmbed::new().title("Owner List").description(owners_text)).ephemeral(true);
        ctx.send(response).await?;
    }

    Ok(())
}
