use chrono::{DateTime, FixedOffset};
use full_palette::{GREY_400, GREY_800};
use image::{ImageBuffer, RgbImage};
use mysql::prelude::*;
use mysql::*;
use plotters::backend::RGBPixel;
use plotters::prelude::*;
use poise::{serenity_prelude::CreateAttachment, CreateReply};
use std::sync::Arc;
use std::{io::Cursor, vec};
use tera::Tera;
use tokio::sync::Mutex;

use crate::utils::svg::create_user_rating::Theme;
use crate::{
    scraping::contest_type::ContestType,
    utils::{svg::create_user_rating::CreateUserRating, svg_to_png::svg_to_png},
    Context, Error,
};

#[derive(Debug, poise::ChoiceParameter)]
pub enum AtCoderContestType {
    #[name = "Algorithm"]
    Algorithm,
    #[name = "Heuristic"]
    Heuristic,
}

#[poise::command(prefix_command, slash_command, subcommands("now", "rating_history"))]
pub async fn rating(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Display the latest rating for a specified AtCoder user.
#[poise::command(prefix_command, slash_command)]
pub async fn now(
    ctx: Context<'_>,
    #[description = "contest_type"] contest_type: AtCoderContestType,
    #[description = "atcoder_user"] atcoder_user: Option<String>,
) -> Result<(), Error> {
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();
    let users: Vec<String> = conn
        .exec(
            "SELECT atcoder_username FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
            params! {"discord_id" => ctx.author().id.to_string().parse::<u64>().unwrap(),
            "server_id" => guild_id},
        )
        .unwrap();

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 {
        lang = selected_data[0].as_str();
    }

    if users.is_empty() {
        if lang == "ja" {
            let response =
                CreateReply::default().content("AtCoderアカウントがリンクされていません。`link`コマンドを使用してAtCoderアカウントをリンクしてください。");
            ctx.send(response).await?;
        } else {
            let response =
                CreateReply::default().content("You have not linked your AtCoder account yet. Please use the `link` command to link your AtCoder account.");
            ctx.send(response).await?;
        }
        return Ok(());
    }

    let atcoder_user = match atcoder_user {
        Some(atcoder_user) => atcoder_user,
        None => users[0].clone(),
    };

    let response = {
        let contest_type: ContestType = match contest_type {
            AtCoderContestType::Algorithm => ContestType::Algorithm,
            AtCoderContestType::Heuristic => ContestType::Heuristic,
        };
        let pool: &Pool = &pool.clone();
        let svg_data = CreateUserRating::from_user(&Arc::new(Mutex::new(pool.clone())), atcoder_user, contest_type, 0, 0, Theme::Dark).await;
        let mut tmpl = Tera::default();
        tmpl.add_raw_template("user_rating.svg", include_str!("../../static/img/user_rating.svg")).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("main", &format!("{}{}", &svg_data.circle_svg, &svg_data.text_svg));
        ctx.insert("gradient", &svg_data.gradient_svg);
        CreateReply::default().attachment(CreateAttachment::bytes(
            svg_to_png(&tmpl.render("user_rating.svg", &ctx).unwrap_or_default(), 1336, 100, 1.0, 1.0),
            "rating.png",
        ))
    };

    ctx.send(response).await?;

    Ok(())
}

/// Show the rating history for a specified AtCoder user.
#[poise::command(prefix_command, slash_command, rename = "history")]
pub async fn rating_history(
    ctx: Context<'_>,
    #[description = "atcoder_user_list"] atcoder_user_list: Option<String>,
    #[description = "contest_type"] contest_type: AtCoderContestType,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let pool = ctx.data().conn.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let guild_id = ctx.guild_id().unwrap().get();
    let users: Vec<String> = if atcoder_user_list.clone().unwrap_or("".to_string()) == "all" {
        conn.exec(
            "SELECT atcoder_username FROM users WHERE server_id=:server_id",
            params! {"server_id" => guild_id},
        )
        .unwrap()
    } else {
        conn.exec(
            "SELECT atcoder_username FROM users WHERE discord_id=:discord_id AND server_id=:server_id",
            params! {"discord_id" => ctx.author().id.to_string().parse::<u64>().unwrap(),
            "server_id" => guild_id},
        )
        .unwrap()
    };

    let selected_data: Vec<String> = conn.exec(
        r"SELECT language FROM server_settings WHERE server_id=:server_id",
        params! {"server_id" => guild_id},
    )?;
    let mut lang = "ja";
    if selected_data.len() == 1 && atcoder_user_list.is_none() {
        lang = selected_data[0].as_str();
    }

    if users.is_empty() {
        if lang == "ja" {
            let response =
                CreateReply::default().content("AtCoderアカウントがリンクされていません。`link`コマンドを使用してAtCoderアカウントをリンクしてください。");
            ctx.send(response).await?;
        } else {
            let response =
                CreateReply::default().content("You have not linked your AtCoder account yet. Please use the `link` command to link your AtCoder account.");
            ctx.send(response).await?;
        }
        return Ok(());
    }

    let atcoder_user_list = match atcoder_user_list {
        Some(atcoder_user_list) => {
            if atcoder_user_list == "all" {
                users.join(",")
            } else {
                atcoder_user_list
            }
        }
        None => users[0].clone(),
    };
    let bg_colors = [
        RGBColor(216, 216, 216),
        RGBColor(216, 197, 178),
        RGBColor(178, 216, 178),
        RGBColor(178, 236, 236),
        RGBColor(178, 178, 255),
        RGBColor(236, 236, 178),
        RGBColor(255, 216, 178),
        RGBColor(255, 178, 178),
    ];
    let circle_bg_colors = [
        RGBColor(128, 128, 128),
        RGBColor(128, 64, 0),
        RGBColor(0, 128, 0),
        RGBColor(0, 192, 192),
        RGBColor(0, 0, 255),
        RGBColor(192, 192, 0),
        RGBColor(255, 128, 0),
        RGBColor(255, 0, 0),
    ];

    let response = {
        let contest_type: ContestType = match contest_type {
            AtCoderContestType::Algorithm => ContestType::Algorithm,
            AtCoderContestType::Heuristic => ContestType::Heuristic,
        };
        let contest_type = contest_type as i8;
        let image_width = 1280;
        let image_height = 720;
        let mut buffer: Vec<u8> = vec![0; image_width * image_height * 3];
        {
            let root = BitMapBackend::<RGBPixel>::with_buffer(&mut buffer, (image_width as u32, image_height as u32)).into_drawing_area();

            root.fill(&WHITE)?;

            let mut y_min = 99999999;
            let mut y_max = 0;
            let mut x_min = chrono::Utc::now().into();
            let x_max = chrono::DateTime::parse_from_str("2015-04-11 21:00:00+0900", "%Y-%m-%d %H:%M:%S%z");
            let mut x_max = x_max.unwrap();

            let caption = "Rating History";
            let font = ("Lato", 40);

            let mut point_series_vec = vec![];
            let mut line_series_vec = vec![];

            let user_list: Vec<&str> = atcoder_user_list.split(',').collect();
            for (idx, atcoder_user) in user_list.clone().into_iter().enumerate() {
                let atcoder_rating: Vec<(i32, String, i32)> = conn
                    .exec(
                        "SELECT
                            user_ratings.rating,
                            contests.start_time,
                            contests.duration
                        FROM
                            user_ratings
                        JOIN
                            contests
                        ON
                            contests.contest_id = user_ratings.contest
                        WHERE user_ratings.user_name=:atcoder_id and user_ratings.type=:contest_type",
                        params! {"atcoder_id" => atcoder_user, "contest_type" => contest_type},
                    )
                    .unwrap();
                let mut xs = vec![];
                let mut ys = vec![];
                for i in atcoder_rating {
                    let start_time = chrono::DateTime::parse_from_str(&i.1, "%Y-%m-%d %H:%M:%S%z").unwrap();
                    let offset = chrono::Duration::minutes(i.2 as i64);
                    xs.push(start_time + offset);
                    ys.push(i.0);
                }

                let (y_min_temp, y_max_temp) = ys.iter().fold((ys[0], ys[0]), |(m, n), v| (std::cmp::min(*v, m), std::cmp::max(*v, n)));
                let y_min_temp = y_min_temp / 400 * 400;
                let y_min_temp = std::cmp::max(0, y_min_temp - 50);
                let y_max_temp = y_max_temp / 400 * 400 + 450;
                let x_min_temp = *xs.first().unwrap() - (*xs.last().unwrap() - *xs.first().unwrap()) / 20;
                let x_max_temp = *xs.last().unwrap() + (*xs.last().unwrap() - *xs.first().unwrap()) / 20;

                y_min = std::cmp::min(y_min, y_min_temp);
                y_max = std::cmp::max(y_max, y_max_temp);
                x_min = std::cmp::min(x_min, x_min_temp);
                x_max = std::cmp::max(x_max, x_max_temp);
                if user_list.len() == 1 {
                    let point_series = xs
                        .iter()
                        .zip(ys.iter())
                        .map(|(x, y)| {
                            EmptyElement::at((*x, *y))
                                + Circle::new(
                                    (0, 0),
                                    4,
                                    ShapeStyle::from(&circle_bg_colors[std::cmp::min(y / 400, (circle_bg_colors.len() - 1) as i32) as usize]).filled(),
                                )
                                + Circle::new((0, 0), 4, GREY_400.stroke_width(1))
                        })
                        .collect::<Vec<_>>();
                    point_series_vec.push(point_series);
                } else {
                    let point_series = xs
                        .iter()
                        .zip(ys.iter())
                        .map(|(x, y)| {
                            EmptyElement::at((*x, *y))
                                + Circle::new((0, 0), 4, HSLColor(idx as f64 / user_list.len() as f64, 1.0, 0.5).filled())
                                + Circle::new((0, 0), 4, TRANSPARENT)
                        })
                        .collect::<Vec<_>>();
                    point_series_vec.push(point_series);
                };
                if user_list.len() == 1 {
                    let line_series = LineSeries::new(xs.iter().zip(ys.iter()).map(|(x, y)| (*x, *y)), GREY_800);
                    line_series_vec.push(line_series);
                } else {
                    let line_series = LineSeries::new(
                        xs.iter().zip(ys.iter()).map(|(x, y)| (*x, *y)),
                        HSLColor(idx as f64 / user_list.len() as f64, 1.0, 0.5),
                    );
                    line_series_vec.push(line_series);
                };
            }

            let mut chart = ChartBuilder::on(&root)
                .caption(caption, font.into_font())
                .margin(40)
                .x_label_area_size(32)
                .y_label_area_size(84)
                .build_cartesian_2d(x_min..x_max, y_min..y_max)?;
            chart.configure_mesh().x_label_formatter(&|x: &DateTime<FixedOffset>| x.format("%Y/%m/%d").to_string()).draw()?;

            chart.draw_series((0..8).map(|index: i32| {
                Rectangle::new(
                    [
                        (x_min - (x_max - x_min) / 20, 400 * index),
                        (
                            x_max + (x_max - x_min) / 20,
                            match index {
                                7 => 30000,
                                _ => 400 * index + 400,
                            },
                        ),
                    ],
                    ShapeStyle::from(&bg_colors[index as usize]).filled(),
                )
            }))?;
            for i in 0..100 {
                chart.draw_series(LineSeries::new(
                    [(x_min - (x_max - x_min) / 20, 400 * i), (x_max + (x_max - x_min) / 20, 400 * i)],
                    WHITE,
                ))?;
            }
            let user_list: Vec<&str> = atcoder_user_list.split(',').collect();
            let length = line_series_vec.len();
            for (idx, data) in line_series_vec.into_iter().enumerate() {
                let idx_clone = idx;
                let user_name = user_list[idx];
                let user_list_clone = user_list.clone();
                if length == 1 {
                    chart.draw_series(data)?.label(user_name).legend(move |(x, y)| {
                        PathElement::new(
                            vec![(x, y), (x + 20, y)],
                            GREY_800, // Use the cloned user_list
                        )
                    });
                } else {
                    chart.draw_series(data)?.label(user_name).legend(move |(x, y)| {
                        PathElement::new(
                            vec![(x, y), (x + 20, y)],
                            HSLColor(idx_clone as f64 / user_list_clone.len() as f64, 1.0, 0.5), // Use the cloned user_list
                        )
                    });
                }
            }
            for i in point_series_vec {
                chart.draw_series(i)?;
            }
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::UpperLeft)
                .border_style(BLACK)
                .background_style(WHITE.mix(0.8))
                .label_font(("Lato", 20))
                .draw()
                .unwrap();
        }

        let img: RgbImage = ImageBuffer::from_raw(image_width as u32, image_height as u32, buffer).expect("Failed to create image buffer");

        let mut png_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut png_data);
            img.write_to(&mut cursor, image::ImageFormat::Png)?;
        }
        CreateReply::default().attachment(CreateAttachment::bytes(png_data, "history.png"))
    };

    ctx.send(response).await?;

    Ok(())
}
