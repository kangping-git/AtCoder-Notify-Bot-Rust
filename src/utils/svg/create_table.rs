use std::{collections::BTreeSet, sync::Arc};

use fontdb::{Database, Query, Source};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use fontdue::Font;
use mysql::Pool;
use tera::{Context, Tera};
use tokio::sync::Mutex;

use crate::scraping::contest_type::ContestType;

use super::create_user_rating::{CreateUserRating, Theme};

#[derive(Debug, Clone)]
pub enum Row {
    Rating(TableRowsRating),
    Text(TableRowsText),
}
#[derive(Debug, Clone)]
pub enum Align {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone)]
pub struct UserRating {
    pub color_theme: Theme,
    pub username: String,
    pub contest_type: ContestType,
}
#[derive(Debug, Clone)]
pub struct RatingCustom {
    pub color_theme: Theme,
    pub rating: i32,
    pub has_bronze: bool,
    pub title: String,
}
#[derive(Debug, Clone)]
pub enum RatingType {
    UserRating(UserRating),
    Custom(RatingCustom),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Title {
    RatingCustom(RatingCustom),
    UserRating(UserRating),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct TableRowsRating {
    pub title: Title,
    pub width: i32,
    pub data: Vec<RatingType>,
}

#[derive(Debug, Clone)]
pub struct TextConfig {
    pub value: String,
    pub color: String,
}
#[derive(Debug, Clone)]
pub struct TableRowsText {
    pub title: Title,
    pub width: i32,
    pub align: Align,
    pub data: Vec<TextConfig>,
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub svg: String,
    pub width: i32,
    pub height: i32,
}

pub async fn create_table(pool: &Arc<Mutex<Pool>>, title: String, table_rows: Vec<Row>) -> TableData {
    let mut db = Database::new();
    db.load_system_fonts();

    let query = Query {
        families: &[fontdb::Family::Name("Lato")],
        weight: fontdb::Weight::BOLD,
        ..Default::default()
    };
    let id = db.query(&query).unwrap();
    let face = db.face(id).unwrap();
    let font_data = match &face.source {
        Source::Binary(data) => data.as_ref().as_ref(),
        Source::File(path) => &std::fs::read(path).unwrap_or_else(|_| panic!("Error loading font data from file: {:?}", path)),
        err => panic!("Error loading font data. {:?}", err),
    };

    let font = Font::from_bytes(font_data, fontdue::FontSettings::default()).expect("Error loading font");
    let scale = 70.0;

    let mut gradient_vec: Vec<String> = vec![];
    let mut circle_vec: Vec<String> = vec![];
    let mut rows_vec: Vec<String> = vec![];
    let mut x = 0;
    let mut gradient_id_set: BTreeSet<String> = BTreeSet::new();
    let mut height = 0;
    for row in table_rows {
        let mut y = 160;
        let mut text_svg_data: Vec<String> = vec![];
        match row {
            Row::Rating(rating_data) => {
                match rating_data.title {
                    Title::Text(text) => {
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{y}\" fill=\"white\" font-weight=\"bold\" text-anchor=\"middle\" font-size=\"70\">{}</text>",
                            x + rating_data.width / 2,
                            text
                        ));
                    }
                    Title::UserRating(user) => {
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&user.username, scale, 0));

                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32) + 120.0;

                        let x = x + rating_data.width / 2 - (width / 2.0) as i32;

                        let rating = CreateUserRating::from_user(pool, user.username.clone(), user.contest_type, x + 5, y - 78, user.color_theme).await;
                        if !gradient_id_set.contains(&rating.option.gradient_name) {
                            gradient_vec.push(rating.gradient_svg);
                            gradient_id_set.insert(rating.option.gradient_name);
                        }

                        circle_vec.push(rating.circle_svg);
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{y}\" fill=\"{}\" font-size=\"70\">{}</text>",
                            x + 120,
                            rating.option.border_color,
                            &user.username
                        ));
                    }
                    Title::RatingCustom(custom_data) => {
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&custom_data.title, scale, 0));
                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32) + 120.0;
                        let x = x + rating_data.width / 2 - (width / 2.0) as i32;

                        let rating = CreateUserRating::from_number(
                            custom_data.title.clone(),
                            custom_data.rating,
                            x + 5,
                            y - 78,
                            custom_data.has_bronze,
                            custom_data.color_theme,
                        )
                        .await;
                        if !gradient_id_set.contains(&rating.option.gradient_name) {
                            gradient_vec.push(rating.gradient_svg);
                            gradient_id_set.insert(rating.option.gradient_name);
                        }
                        circle_vec.push(rating.circle_svg);
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"70\">{}</text>",
                            x + 120,
                            y,
                            rating.option.border_color,
                            &custom_data.title
                        ));
                    }
                }
                y += 110;
                for records in rating_data.data {
                    match records {
                        RatingType::UserRating(user_data) => {
                            let rating =
                                CreateUserRating::from_user(pool, user_data.username.clone(), user_data.contest_type, x + 5, y - 78, user_data.color_theme)
                                    .await;
                            if !gradient_id_set.contains(&rating.option.gradient_name) {
                                gradient_vec.push(rating.gradient_svg);
                                gradient_id_set.insert(rating.option.gradient_name);
                            }
                            circle_vec.push(rating.circle_svg);
                            text_svg_data.push(format!(
                                "<text x=\"{}\" y=\"{y}\" fill=\"{}\" font-size=\"70\">{}</text>",
                                x + 120,
                                rating.option.border_color,
                                &user_data.username
                            ));
                        }
                        RatingType::Custom(custom_data) => {
                            let rating = CreateUserRating::from_number(
                                custom_data.title.clone(),
                                custom_data.rating,
                                x + 5,
                                y - 78,
                                custom_data.has_bronze,
                                custom_data.color_theme,
                            )
                            .await;
                            if !gradient_id_set.contains(&rating.option.gradient_name) {
                                gradient_vec.push(rating.gradient_svg);
                                gradient_id_set.insert(rating.option.gradient_name);
                            }
                            circle_vec.push(rating.circle_svg);
                            text_svg_data.push(format!(
                                "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"70\">{}</text>",
                                x + 120,
                                y,
                                rating.option.border_color,
                                &custom_data.title
                            ));
                        }
                    }
                    y += 110;
                }
                x += rating_data.width + 30;
            }
            Row::Text(text_data) => {
                let offset = match text_data.align {
                    Align::Start => 0,
                    Align::Middle => text_data.width / 2,
                    Align::End => text_data.width,
                };
                let text_align = match text_data.align {
                    Align::Start => "start",
                    Align::Middle => "middle",
                    Align::End => "end",
                };
                match text_data.title {
                    Title::Text(text) => {
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{y}\" fill=\"white\" font-weight=\"bold\" text-anchor=\"middle\" font-size=\"70\">{}</text>",
                            x + text_data.width / 2,
                            text
                        ));
                    }
                    Title::UserRating(user) => {
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&user.username, scale, 0));

                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32) + 120.0;
                        let x = x + text_data.width / 2 - (width / 2.0) as i32;

                        let rating = CreateUserRating::from_user(pool, user.username.clone(), user.contest_type, x + 5, y - 78, user.color_theme).await;
                        if !gradient_id_set.contains(&rating.option.gradient_name) {
                            gradient_vec.push(rating.gradient_svg);
                            gradient_id_set.insert(rating.option.gradient_name);
                        }
                        circle_vec.push(rating.circle_svg);
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{y}\" fill=\"{}\" font-size=\"70\">{}</text>",
                            x + 120,
                            rating.option.border_color,
                            &user.username
                        ));
                    }
                    Title::RatingCustom(custom_data) => {
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.append(&[font.clone()], &TextStyle::new(&custom_data.title, scale, 0));

                        let width = layout.glyphs().last().map_or(0.0, |g| g.x + g.width as f32) + 120.0;
                        let x = x + text_data.width / 2 - (width / 2.0) as i32;

                        let rating = CreateUserRating::from_number(
                            custom_data.title.clone(),
                            custom_data.rating,
                            x + 5,
                            y - 78,
                            custom_data.has_bronze,
                            custom_data.color_theme,
                        )
                        .await;
                        if !gradient_id_set.contains(&rating.option.gradient_name) {
                            gradient_vec.push(rating.gradient_svg);
                            gradient_id_set.insert(rating.option.gradient_name);
                        }
                        circle_vec.push(rating.circle_svg);
                        text_svg_data.push(format!(
                            "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"70\">{}</text>",
                            x + 120,
                            y,
                            rating.option.border_color,
                            &custom_data.title
                        ));
                    }
                }
                y += 110;
                for records in text_data.data {
                    text_svg_data.push(format!(
                        "<text x=\"{}\" y=\"{y}\" fill=\"{}\" text-anchor=\"{text_align}\" font-size=\"70\">{}</text>",
                        x + offset,
                        records.color,
                        records.value
                    ));
                    y += 110;
                }
                x += text_data.width + 30;
            }
        }
        rows_vec.push(text_svg_data.join(""));
        height = std::cmp::max(height, y);
    }
    let mut tmpl = Tera::default();
    tmpl.add_raw_templates(vec![("table.svg", include_str!("../../../static/img/table.svg"))]).unwrap();
    let mut ctx = Context::new();
    ctx.insert("gradient", &gradient_vec.join(""));
    ctx.insert("rows", &rows_vec.join(""));
    ctx.insert("circles", &circle_vec.join(""));
    ctx.insert("width", &x);
    ctx.insert("height", &height);
    ctx.insert("title", &title);
    ctx.insert("title_x", &(x / 2));
    TableData {
        svg: tmpl.render("table.svg", &ctx).unwrap(),
        width: x,
        height,
    }
}
