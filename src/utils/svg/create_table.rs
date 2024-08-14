use std::{collections::BTreeSet, sync::Arc};

use mysql::Pool;
use tera::{Context, Tera};
use tokio::sync::Mutex;

use crate::scraping::contest_type::ContestType;

use super::create_user_rating::CreateUserRating;

#[derive(Debug)]
pub enum Row {
    Rating(TableRowsRating),
    Text(TableRowsText),
}
#[derive(Debug)]
pub enum Align {
    Start,
    Middle,
    End,
}

#[derive(Debug)]
pub struct UserRating {
    pub username: String,
    pub contest_type: ContestType,
}
#[derive(Debug)]
pub struct RatingCustom {
    pub rating: i32,
    pub title: String,
}
#[derive(Debug)]
pub enum RatingType {
    UserRating(UserRating),
    Custom(RatingCustom),
}
#[derive(Debug)]
pub struct TableRowsRating {
    pub title: String,
    pub width: i32,
    pub data: Vec<RatingType>,
}

#[derive(Debug)]
pub struct TextConfig {
    pub value: String,
    pub color: String,
}
#[derive(Debug)]
pub struct TableRowsText {
    pub title: String,
    pub width: i32,
    pub align: Align,
    pub data: Vec<TextConfig>,
}

#[derive(Debug)]
pub struct TableData {
    pub svg: String,
    pub width: i32,
    pub height: i32,
}

pub async fn create_table(
    pool: &Arc<Mutex<Pool>>,
    title: String,
    table_rows: Vec<Row>,
) -> TableData {
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
                text_svg_data.push(format!(
                    "<text x=\"{}\" y=\"{y}\" fill=\"white\" font-weight=\"bold\" text-anchor=\"middle\" font-size=\"70\">{}</text>",
                    x + rating_data.width / 2,
                    rating_data.title
                ));
                y += 110;
                for records in rating_data.data {
                    match records {
                        RatingType::UserRating(user_data) => {
                            let rating = CreateUserRating::from_user(
                                pool,
                                user_data.username.clone(),
                                user_data.contest_type,
                                x + 5,
                                y - 78,
                            )
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
                x += rating_data.width;
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
                text_svg_data.push(format!(
                    "<text x=\"{}\" y=\"{y}\" fill=\"white\" font-weight=\"bold\" text-anchor=\"middle\" font-size=\"70\">{}</text>",
                    x + text_data.width / 2,
                    text_data.title
                ));
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
                x += text_data.width;
            }
        }
        rows_vec.push(text_svg_data.join(""));
        height = std::cmp::max(height, y);
    }
    let mut tmpl = Tera::default();
    tmpl.add_raw_templates(vec![(
        "table.svg",
        include_str!("../../../static/img/table.svg"),
    )])
    .unwrap();
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
