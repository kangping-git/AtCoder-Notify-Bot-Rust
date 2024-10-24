use std::{collections::BTreeMap, env, fs, sync::Arc};

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
    scraping::contest_type::ContestType,
    utils::svg::create_user_rating::{CreateUserRating, Theme},
};
use actix_web::web::Bytes;

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
    real_performance: i32,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct RatingData {
    algo_avg: f64,
    algo_stddev: f64,
    heuristic_avg: f64,
    heuristic_stddev: f64,
    algo_inner_avg: f64,
    algo_inner_stddev: f64,
    heuristic_inner_avg: f64,
    heuristic_inner_stddev: f64,
}

#[get("/")]
async fn home() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/index.html").to_string()
    } else {
        fs::read_to_string("static/pages/src/index.html").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType::html()).body(file_content)
}
#[get("/supporters/")]
async fn supporters() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/supporters.html").to_string()
    } else {
        fs::read_to_string("static/pages/src/supporters.html").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType::html()).body(file_content)
}
#[get("/418/")]
async fn im_a_teapot() -> impl Responder {
    let file_content = include_str!("../static/pages/src/418.html").to_string();
    HttpResponse::ImATeapot().content_type(ContentType::html()).body(file_content)
}
#[get("/deviation/")]
async fn deviation() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/deviation.html").to_string()
    } else {
        fs::read_to_string("static/pages/src/deviation.html").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType::html()).body(file_content)
}
#[get("/rating_simulator/")]
async fn rating_simulator() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/rating_simulator.html").to_string()
    } else {
        fs::read_to_string("static/pages/src/rating_simulator.html").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType::html()).body(file_content)
}
#[get("/rating_other_contests/")]
async fn rating_other_contests() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/rating_other_contests.html").to_string()
    } else {
        fs::read_to_string("static/pages/src/rating_other_contests.html").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType::html()).body(file_content)
}
#[get("/output.css")]
async fn output_css() -> impl Responder {
    let file_content = if env::var("DEBUG").is_err() {
        include_str!("../static/pages/src/output.css").to_string()
    } else {
        fs::read_to_string("static/pages/src/output.css").unwrap()
    };
    HttpResponse::Ok().content_type(ContentType(mime::TEXT_CSS)).body(file_content)
}

#[get("/notify_icon.svg")]
async fn icon() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType(mime::IMAGE_SVG)).body(include_str!("../static/img/notify_icon.svg"))
}

#[get("/supporters/jikky1618.jpg")]
async fn supporter_jikky() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType(mime::IMAGE_JPEG)).body(Bytes::from_static(include_bytes!("../static/img/jikky1618.jpg")))
}
#[get("/supporters/person.png")]
async fn supporter_person() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType(mime::IMAGE_PNG)).body(Bytes::from_static(include_bytes!("../static/img/person.png")))
}
#[get("/notify_icon_white.svg")]
async fn icon_white() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType(mime::IMAGE_SVG)).body(include_str!("../static/img/notify_icon_white.svg"))
}

#[get("/api/atcoder/rating/{atcoder_id}")]
async fn get_rating(pool: web::Data<Pool>, id: web::Path<String>) -> HttpResponse {
    let mut conn = pool.get_conn().unwrap();
    let atcoder_rating: Vec<(i32, i32, u32, u32)> = conn
        .exec(
            "select algo_rating, heuristic_rating, algo_contests, heuristic_contests from atcoder_user_ratings where user_name=:atcoder_id",
            params! {"atcoder_id" => id.to_string()},
        )
        .unwrap();
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
        HttpResponse::Ok().content_type(ContentType::json()).body(data)
    } else {
        let rating_data = UserRatingsNotFound {
            user_id: id.to_string(),
            found: false,
        };
        let data = serde_json::to_string(&rating_data).unwrap();
        HttpResponse::Ok().content_type(ContentType::json()).body(data)
    }
}

#[get("/api/atcoder/image/{atcoder_id}")]
async fn get_user_image(pool: web::Data<Pool>, id: web::Path<String>, query: web::Query<BTreeMap<String, String>>) -> HttpResponse {
    let algo = "algo".to_string();
    let contest_type_str = query.get("contest_type").unwrap_or(&algo);
    let contest_type: ContestType = if contest_type_str == "heuristic" {
        ContestType::Heuristic
    } else {
        ContestType::Algorithm
    };
    let pool: &Pool = pool.as_ref();
    let svg_data = CreateUserRating::from_user(&Arc::new(Mutex::new(pool.clone())), id.to_string(), contest_type, 0, 0, Theme::Light).await;
    let mut tmpl = Tera::default();
    tmpl.add_raw_template("user_rating.svg", include_str!("../static/img/user_rating.svg")).unwrap();
    let mut ctx = Context::new();
    ctx.insert("main", &format!("{}{}", &svg_data.circle_svg, &svg_data.text_svg));
    ctx.insert("gradient", &svg_data.gradient_svg);
    HttpResponse::Ok().content_type("image/svg+xml").body(tmpl.render("user_rating.svg", &ctx).unwrap())
}

#[get("/api/atcoder/data/rating.json")]
async fn data_rating(pool: web::Data<Pool>) -> HttpResponse {
    let mut conn = pool.get_conn().unwrap();
    let (algo_avg,algo_inner_avg, algo_stddev,algo_inner_stddev, heuristic_avg,heuristic_inner_avg, heuristic_stddev,heuristic_inner_stddev) = conn
        .query(
            "SELECT 
                (SELECT AVG(algo_rating) FROM atcoder_user_ratings WHERE algo_contests > 0),
                (SELECT AVG(IF(algo_rating <= 400, 400 - 400 * LOG(400 / algo_rating), algo_rating)) FROM atcoder_user_ratings WHERE algo_contests > 0),
                (SELECT STDDEV(algo_rating) FROM atcoder_user_ratings WHERE algo_contests > 0),
                (SELECT STDDEV(IF(algo_rating <= 400, 400 - 400 * LOG(400 / algo_rating), algo_rating)) FROM atcoder_user_ratings WHERE algo_contests > 0),
                (SELECT AVG(heuristic_rating) FROM atcoder_user_ratings WHERE heuristic_contests > 0),
                (SELECT AVG(IF(heuristic_rating <= 400, 400 - 400 * LOG(400 / heuristic_rating), heuristic_rating)) FROM atcoder_user_ratings WHERE heuristic_contests > 0),
                (SELECT STDDEV(heuristic_rating) FROM atcoder_user_ratings WHERE heuristic_contests > 0),
                (SELECT STDDEV(IF(heuristic_rating <= 400, 400 - 400 * LOG(400 / heuristic_rating), heuristic_rating)) FROM atcoder_user_ratings WHERE heuristic_contests > 0)",
        )
        .unwrap()[0];
    let data = RatingData {
        algo_avg,
        algo_stddev,
        heuristic_avg,
        heuristic_stddev,
        algo_inner_avg,
        algo_inner_stddev,
        heuristic_inner_avg,
        heuristic_inner_stddev,
    };
    HttpResponse::Ok().content_type(ContentType::json()).body(serde_json::to_string(&data).unwrap())
}

#[get("/api/atcoder/history/{atcoder_id}")]
async fn get_history(pool: web::Data<Pool>, id: web::Path<String>, query: web::Query<BTreeMap<String, String>>) -> HttpResponse {
    let algo = "algo".to_string();
    let contest_type_str = query.get("contest_type").unwrap_or(&algo);
    let contest_type: i32 = if contest_type_str == "heuristic" { 1 } else { 0 };
    let mut conn = pool.get_conn().unwrap();
    let atcoder_rating: Vec<(String, i32, i32, i32, String, i32)> = conn
        .exec(
            "SELECT
            user_ratings.contest, user_ratings.performance, LEAST(user_ratings.performance,contests.rating_range_end+401), user_ratings.rating,
            contests.start_time,
            contests.duration
        FROM
            user_ratings
        JOIN
            contests
        ON
            contests.contest_id = user_ratings.contest
        WHERE user_ratings.user_name=:atcoder_id and user_ratings.type=:contest_type ORDER BY contests.start_time",
            params! {"atcoder_id" => id.to_string(), "contest_type" => contest_type},
        )
        .unwrap();
    if !atcoder_rating.is_empty() {
        let mut history = vec![];
        for i in atcoder_rating {
            history.push(UserHistoryElement {
                contest_id: i.0,
                rating: i.3,
                real_performance: i.1,
                performance: i.2,
                start_time: i.4,
                duration: i.5,
            })
        }
        let history_data = UserHistory {
            user_id: id.to_string(),
            found: true,
            history,
        };
        let data = serde_json::to_string(&history_data).unwrap();
        HttpResponse::Ok().content_type(ContentType::json()).body(data)
    } else {
        let rating_data = UserHistory {
            user_id: id.to_string(),
            found: false,
            history: vec![],
        };
        let data = serde_json::to_string(&rating_data).unwrap();
        HttpResponse::Ok().content_type(ContentType::json()).body(data)
    }
}

async fn default_handler(req_method: Method) -> Result<impl Responder> {
    match req_method {
        Method::GET => {
            let file_content = if env::var("DEBUG").is_err() {
                include_str!("../static/pages/src/404.html").to_string()
            } else {
                fs::read_to_string("static/pages/src/404.html").unwrap()
            };
            let file = HttpResponse::NotFound().content_type(ContentType::html()).body(file_content);
            Ok(Either::Left(file))
        }
        _ => Ok(Either::Right(HttpResponse::MethodNotAllowed().finish())),
    }
}

#[actix_web::main]
pub async fn start() {
    for item in dotenvy::dotenv_iter().unwrap() {
        let (key, val) = item.unwrap();
        env::set_var(key, val);
    }

    log::info!("Web Server Service");
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        std::env::var("MYSQL_USER").expect(""),
        std::env::var("MYSQL_PASS").expect(""),
        std::env::var("MYSQL_HOST").expect(""),
        std::env::var("MYSQL_PORT").expect(""),
        std::env::var("MYSQL_DATABASE").expect("")
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
            .service(output_css)
            .service(data_rating)
            .service(deviation)
            .service(icon_white)
            .service(rating_simulator)
            .service(im_a_teapot)
            .service(supporters)
            .service(supporter_jikky)
            .service(supporter_person)
            .service(rating_other_contests)
            .default_service(web::to(default_handler))
    })
    .bind(("127.0.0.1", port.parse::<u16>().unwrap()))
    .unwrap()
    .run()
    .await;
}
