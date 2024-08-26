use std::sync::Arc;

use mysql::prelude::*;
use mysql::*;
use tera::{Context, Tera};
use tokio::sync::Mutex;

use crate::scraping::contest_type;

#[allow(unused)]
pub struct UserRatingOption {
    pub rating: i32,
    pub border_color: String,
    pub text: String,
    pub gradient_name: String,
}

pub struct CreateUserRating {
    pub gradient_svg: String,
    pub circle_svg: String,
    pub text_svg: String,
    pub option: UserRatingOption,
}
#[derive(Debug, Clone)]
pub enum Theme {
    Light,
    Dark,
}

impl CreateUserRating {
    pub async fn from_user(pool: &Arc<Mutex<Pool>>, user: String, contest_type: contest_type::ContestType, x: i32, y: i32, theme: Theme) -> CreateUserRating {
        let pool = pool.lock().await;
        let contest_type: i32 = contest_type as i32;
        let mut conn = pool.get_conn().unwrap();
        let atcoder_rating: Vec<(i32, i32, u32, u32)> = conn
            .exec(
                "select algo_rating, heuristic_rating, algo_contests, heuristic_contests from atcoder_user_ratings where user_name=:atcoder_id",
                params! {"atcoder_id" => user.clone()},
            )
            .unwrap();
        let rating;
        if !atcoder_rating.is_empty() {
            if contest_type == 0 {
                rating = atcoder_rating[0].0;
            } else {
                rating = atcoder_rating[0].1;
            }
        } else {
            rating = 0
        }
        let rating_colors = if let Theme::Light = theme {
            &[
                "#404040", "#808080", "#804000", "#008000", "#00C0C0", "#0000FF", "#C0C000", "#FF8000", "#FF0000",
            ]
        } else {
            &[
                "#FFFFFF", "#C0C0C0", "#B08C56", "#3FAF3F", "#42E0E0", "#8888FF", "#FFFF56", "#FFB836", "#FF6767",
            ]
        };
        let stroke_colors = if let Theme::Light = theme {
            &[
                "#404040",
                "#808080",
                "#804000",
                "#008000",
                "#00C0C0",
                "#0000FF",
                "#C0C000",
                "#FF8000",
                "#FF0000",
                "rgb(128, 128, 128)",
                "rgb(255, 215, 0)",
            ]
        } else {
            &[
                "#FFFFFF",
                "#C0C0C0",
                "#B08C56",
                "#3FAF3F",
                "#42E0E0",
                "#8888FF",
                "#FFFF56",
                "#FFB836",
                "#FF6767",
                "rgb(128, 128, 128)",
                "rgb(255, 215, 0)",
            ]
        };
        let percent = ((rating % 400) as f64) / 4.0;
        let mut fill = format!("#gradient_user_rating_{}", rating);
        if rating >= 3600 {
            fill = "#Gold".to_string()
        } else if rating >= 3200 {
            fill = "#Silver".to_string()
        }
        let mut tmpl = Tera::default();
        tmpl.add_raw_templates(vec![
            ("user_rating_gradient.svg", include_str!("../../../static/img/user_rating_gradient.svg")),
            ("user_rating_circle.svg", include_str!("../../../static/img/user_rating_circle.svg")),
            ("user_rating_text.svg", include_str!("../../../static/img/user_rating_text.svg")),
        ])
        .unwrap();
        let mut ctx = Context::new();
        ctx.insert("percent", &percent);
        ctx.insert("user_name", &format!("gradient_user_rating_{}", rating));
        ctx.insert(
            "rating_color",
            &rating_colors[std::cmp::min(((rating + 399) / 400) as usize, rating_colors.len() - 1)],
        );
        ctx.insert(
            "stroke_color",
            &stroke_colors[std::cmp::min(((rating + 399) / 400) as usize, stroke_colors.len() - 1)],
        );
        ctx.insert("fill_url", &fill);
        ctx.insert("username", &user);
        ctx.insert("x", &(x + 100));
        ctx.insert("y", &(y + 5));
        ctx.insert("cx", &(x + 50));
        ctx.insert("cy", &(y + 50));
        CreateUserRating {
            gradient_svg: tmpl.render("user_rating_gradient.svg", &ctx).unwrap(),
            circle_svg: tmpl.render("user_rating_circle.svg", &ctx).unwrap(),
            text_svg: tmpl.render("user_rating_text.svg", &ctx).unwrap(),
            option: UserRatingOption {
                rating,
                border_color: stroke_colors[std::cmp::min(((rating + 399) / 400) as usize, stroke_colors.len() - 1)].to_string(),
                text: user,
                gradient_name: fill,
            },
        }
    }
    pub async fn from_number(title: String, num: i32, x: i32, y: i32, has_bronze: bool, theme: Theme) -> CreateUserRating {
        let rating = num;
        let rating_colors = if let Theme::Light = theme {
            &[
                "#404040", "#808080", "#804000", "#008000", "#00C0C0", "#0000FF", "#C0C000", "#FF8000", "#FF0000",
            ]
        } else {
            &[
                "#FFFFFF", "#C0C0C0", "#B08C56", "#3FAF3F", "#42E0E0", "#8888FF", "#FFFF56", "#FFB836", "#FF6767",
            ]
        };
        let stroke_colors: &Vec<&str> = if has_bronze {
            if let Theme::Light = theme {
                &vec![
                    "#404040",
                    "#808080",
                    "#804000",
                    "#008000",
                    "#00C0C0",
                    "#0000FF",
                    "#C0C000",
                    "#FF8000",
                    "#FF0000",
                    "rgb(150, 92, 44)",
                    "rgb(128, 128, 128)",
                    "rgb(255, 215, 0)",
                ]
            } else {
                &vec![
                    "#404040",
                    "#808080",
                    "#804000",
                    "#008000",
                    "#00C0C0",
                    "#0000FF",
                    "#C0C000",
                    "#FF8000",
                    "#FF0000",
                    "rgb(128, 128, 128)",
                    "rgb(255, 215, 0)",
                ]
            }
        } else if let Theme::Dark = theme {
            {
                &vec![
                    "#FFFFFF",
                    "#C0C0C0",
                    "#B08C56",
                    "#3FAF3F",
                    "#42E0E0",
                    "#8888FF",
                    "#FFFF56",
                    "#FFB836",
                    "#FF6767",
                    "rgb(128, 128, 128)",
                    "rgb(255, 215, 0)",
                ]
            }
        } else {
            &vec![
                "#404040",
                "#808080",
                "#804000",
                "#008000",
                "#00C0C0",
                "#0000FF",
                "#C0C000",
                "#FF8000",
                "#FF0000",
                "rgb(128, 128, 128)",
                "rgb(255, 215, 0)",
            ]
        };
        let percent = ((rating % 400) as f64) / 4.0;
        let mut fill = format!("#gradient_rating_{}", rating);
        if has_bronze {
            if rating >= 4000 {
                fill = "#Gold".to_string()
            } else if rating >= 3600 {
                fill = "#Silver".to_string()
            } else if rating >= 3200 {
                fill = "#Bronze".to_string()
            }
        } else if rating >= 3600 {
            fill = "#Gold".to_string()
        } else if rating >= 3200 {
            fill = "#Silver".to_string()
        }
        let mut tmpl = Tera::default();
        tmpl.add_raw_templates(vec![
            ("user_rating_gradient.svg", include_str!("../../../static/img/user_rating_gradient.svg")),
            ("user_rating_circle.svg", include_str!("../../../static/img/user_rating_circle.svg")),
            ("user_rating_text.svg", include_str!("../../../static/img/user_rating_text.svg")),
        ])
        .unwrap();
        let mut ctx = Context::new();
        ctx.insert("percent", &percent);
        ctx.insert("user_name", &format!("gradient_rating_{}", rating));
        ctx.insert(
            "rating_color",
            &rating_colors[std::cmp::min(((rating + 399) / 400) as usize, rating_colors.len() - 1)],
        );
        ctx.insert(
            "stroke_color",
            &stroke_colors[std::cmp::min(((rating + 399) / 400) as usize, stroke_colors.len() - 1)],
        );
        ctx.insert("fill_url", &fill);
        ctx.insert("username", &title);
        ctx.insert("x", &(x + 100));
        ctx.insert("y", &(y + 5));
        ctx.insert("cx", &(x + 50));
        ctx.insert("cy", &(y + 50));
        CreateUserRating {
            gradient_svg: tmpl.render("user_rating_gradient.svg", &ctx).unwrap(),
            circle_svg: tmpl.render("user_rating_circle.svg", &ctx).unwrap(),
            text_svg: tmpl.render("user_rating_text.svg", &ctx).unwrap(),
            option: UserRatingOption {
                rating: num,
                border_color: stroke_colors[std::cmp::min(((rating + 399) / 400) as usize, stroke_colors.len() - 1)].to_string(),
                text: title,
                gradient_name: fill,
            },
        }
    }
}
