use std::{collections::BTreeMap, sync::Arc};

use actix_web::{
    get,
    http::{header::ContentType, Method},
    web, App, Either, HttpResponse, HttpServer, Responder, Result,
};
use mysql::prelude::*;
use mysql::*;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use tokio::sync::Mutex;

use crate::{
    scraping::contest_type::ContestType, utils::svg::create_user_rating::CreateUserRating,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct UserRatings {
    user_id: String,
    found: bool,
    algo_rating: i32,
    algo_rated_num: u32,
    heuristic_rating: i32,
    heuristic_rated_num: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct UserHistoryElement {
    contest_id: String,
    rating: i32,
    performance: i32,
    start_time: String,
    duration: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct UserHistory {
    user_id: String,
    found: bool,
    history: Vec<UserHistoryElement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct UserRatingsNotFound {
    user_id: String,
    found: bool,
}

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("../static/pages/src/index.html"))
}
#[get("/output.css")]
async fn outputcss() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS))
        .body(include_str!("../static/pages/src/output.css"))
}

#[get("/notify_icon.svg")]
async fn icon() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_SVG))
        .body(include_str!("../static/img/notify_icon.svg"))
}

#[get("/api/atcoder/rating/{atcoder_id}")]
async fn get_rating(pool: web::Data<Pool>, id: web::Path<String>) -> HttpResponse {
    let mut conn = pool.get_conn().unwrap();
    let atcoder_rating: Vec<(i32, i32, u32, u32)> = conn.exec(
        "select algo_rating, heuristic_rating, algo_contests, heuristic_contests from atcoder_user_ratings where user_name=:atcoder_id",
        params! {"atcoder_id" => id.to_string()},
    ).unwrap();
    if !atcoder_rating.is_empty() {
        let rating_data = UserRatings {
            user_id: id.to_string(),
            found: true,
            algo_rating: atcoder_rating[0].0,
            algo_rated_num: atcoder_rating[0].2,
            heuristic_rating: atcoder_rating[0].1,
            heuristic_rated_num: atcoder_rating[0].3,
        };
        let data = serde_json::to_string(&rating_data).unwrap();
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(data)
    } else {
        let rating_data = UserRatingsNotFound {
            user_id: id.to_string(),
            found: false,
        };
        let data = serde_json::to_string(&rating_data).unwrap();
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(data)
    }
}

#[get("/api/atcoder/image/{atcoder_id}")]
async fn get_user_image(
    pool: web::Data<Pool>,
    id: web::Path<String>,
    query: web::Query<BTreeMap<String, String>>,
) -> HttpResponse {
    let algo = "algo".to_string();
    let contest_type_str = query.get("contest_type").unwrap_or(&algo);
    let contest_type: ContestType = if contest_type_str == "heuristic" {
        ContestType::Heuristic
    } else {
        ContestType::Algorithm
    };
    let pool: &Pool = pool.as_ref();
    let svg_data = CreateUserRating::from_user(
        &Arc::new(Mutex::new(pool.clone())),
        id.to_string(),
        contest_type,
        0,
        0,
    )
    .await;
    let mut tmpl = Tera::default();
    tmpl.add_raw_template(
        "user_rating.svg",
        include_str!("../static/img/user_rating.svg"),
    )
    .unwrap();
    let mut ctx = Context::new();
    ctx.insert(
        "main",
        &format!("{}{}", &svg_data.circle_svg, &svg_data.text_svg),
    );
    ctx.insert("gradient", &svg_data.gradient_svg);
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(tmpl.render("user_rating.svg", &ctx).unwrap())
}

#[get("/api/atcoder/history/{atcoder_id}")]
async fn get_history(
    pool: web::Data<Pool>,
    id: web::Path<String>,
    query: web::Query<BTreeMap<String, String>>,
) -> HttpResponse {
    let algo = "algo".to_string();
    let contest_type_str = query.get("contest_type").unwrap_or(&algo);
    let contest_type: i32 = if contest_type_str == "heuristic" {
        1
    } else {
        0
    };
    let mut conn = pool.get_conn().unwrap();
    let atcoder_rating: Vec<(String, i32, i32, String, i32)> = conn
        .exec(
            "SELECT
            user_ratings.contest, user_ratings.performance, user_ratings.rating,
            contests.start_time,
            contests.duration
        FROM
            user_ratings
        JOIN
            contests
        ON
            contests.contest_id = user_ratings.contest
        WHERE user_ratings.user_name=:atcoder_id and user_ratings.type=:contest_type",
            params! {"atcoder_id" => id.to_string(), "contest_type" => contest_type},
        )
        .unwrap();
    if !atcoder_rating.is_empty() {
        let mut history = vec![];
        for i in atcoder_rating {
            history.push(UserHistoryElement {
                contest_id: i.0,
                rating: i.2,
                performance: i.1,
                start_time: i.3,
                duration: i.4,
            })
        }
        let history_data = UserHistory {
            user_id: id.to_string(),
            found: true,
            history,
        };
        let data = serde_json::to_string(&history_data).unwrap();
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(data)
    } else {
        let rating_data = UserHistory {
            user_id: id.to_string(),
            found: false,
            history: vec![],
        };
        let data = serde_json::to_string(&rating_data).unwrap();
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(data)
    }
}

async fn default_handler(req_method: Method) -> Result<impl Responder> {
    match req_method {
        Method::GET => {
            let file = HttpResponse::NotFound()
                .content_type(ContentType::html())
                .body(include_str!("../static/pages/src/404.html"));
            Ok(Either::Left(file))
        }
        _ => Ok(Either::Right(HttpResponse::MethodNotAllowed().finish())),
    }
}

#[actix_web::main]
pub async fn start() {
    dotenv::dotenv().unwrap();

    log::info!("Web Server Service");
    let url = format!(
        "mysql://{}:{}@localhost:3306/atcoder_notify",
        std::env::var("MYSQL_USER").expect(""),
        std::env::var("MYSQL_PASS").expect("")
    );
    let pool = Pool::new(url.as_str()).unwrap();

    let port: String = std::env::var("PORT").expect("ポートが指定されていません");

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(get_rating)
            .service(get_history)
            .service(get_user_image)
            .service(home)
            .service(icon)
            .service(outputcss)
            .default_service(web::to(default_handler))
    })
    .bind(("127.0.0.1", port.parse::<u16>().unwrap()))
    .unwrap()
    .run()
    .await;
}
