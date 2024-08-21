use poise::{
    serenity_prelude::{Colour, CreateEmbed, EditRole, UserId},
    CreateReply,
};

use crate::{Context, Error};
use mysql::prelude::*;
use mysql::*;

#[poise::command(prefix_command, slash_command, subcommands("create_roles"))]
pub async fn role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn create_roles(ctx: Context<'_>) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    ctx.defer_ephemeral().await?;

    let has_roles: Vec<i64> = conn.exec(r"SELECT id FROM roles WHERE guild_id=:guild_id", params! {"guild_id" => guild_id})?;
    if !has_roles.is_empty() {
        if lang == "ja" {
            let response = CreateReply::default().embed(CreateEmbed::new().title("エラー").description("ロールはすでに作成されています")).ephemeral(true);
            ctx.send(response).await?;
        } else {
            let response = CreateReply::default().embed(CreateEmbed::new().title("Error").description("Roles have already been created.")).ephemeral(true);
            ctx.send(response).await?;
        }
        return Ok(());
    }

    let atcoder_users_vec: Vec<(u64, i64)> = conn.exec(
        r"SELECT
            users.discord_id,
            COALESCE(atcoder_user_ratings.algo_rating, 0) AS algo_rating
        FROM
            users
        LEFT JOIN
            atcoder_user_ratings
        ON
            users.atcoder_username = atcoder_user_ratings.user_name
        WHERE server_id=:server_id",
        params! {"server_id" => guild_id},
    )?;

    let role_colors_and_names: [(&str, (u8, u8, u8)); 9] = [
        ("Black", (64, 64, 64)),
        ("Gray", (128, 128, 128)),
        ("Brown", (128, 64, 0)),
        ("Green", (0, 128, 0)),
        ("Cyan", (0, 192, 192)),
        ("Blue", (0, 0, 255)),
        ("Yellow", (192, 192, 0)),
        ("Orange", (255, 128, 0)),
        ("Red", (255, 0, 0)),
    ];
    let mut transaction = conn.start_transaction(TxOpts::default()).unwrap();

    for i in role_colors_and_names.iter().enumerate() {
        let (name, color) = i.1;
        let guild = ctx.guild_id().unwrap();
        let output = guild.create_role(ctx.http(), EditRole::new().name(*name).colour(Colour::from_rgb(color.0, color.1, color.2))).await?;
        transaction.exec_drop(
            "INSERT INTO roles (guild_id, role_id, role_color) VALUES (:guild_id, :role_id, :role_color)",
            params! {
                "guild_id" => guild_id,
                "role_id" => output.id.get(),
                "role_color" => i.0
            },
        )?;
        for j in &atcoder_users_vec {
            if i.0 == 0 {
                if j.1 == 0 {
                    let user = UserId::new(j.0);
                    let member = guild.member(ctx.http(), user).await?;
                    member.add_role(ctx.http(), output.id).await?;
                }
            } else if i.0 as i64 == j.1 / 400 + 1 && j.1 != 0 {
                let user = UserId::new(j.0);
                let member = guild.member(ctx.http(), user).await?;
                member.add_role(ctx.http(), output.id).await?;
            }
        }
    }

    transaction.commit().unwrap();

    if lang == "ja" {
        let response = CreateReply::default().embed(CreateEmbed::new().title("成功").description("ロールの作成に成功しました。")).ephemeral(true);
        ctx.send(response).await?;
    } else {
        let response = CreateReply::default().embed(CreateEmbed::new().title("Success").description("Role creation successful.")).ephemeral(true);
        ctx.send(response).await?;
    }

    Ok(())
}
