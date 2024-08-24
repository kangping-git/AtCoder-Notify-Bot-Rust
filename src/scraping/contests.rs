use mysql::prelude::*;
use mysql::*;
use scraper::Selector;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use url::Url;

use super::contest_type::{Contest, ContestRatingType, ContestType};
use std::sync::OnceLock;

type ContestDataTuple = (i32, String, String, String, i32, String, i8, i8, i32, i32, String, i8);

#[derive(Debug)]
struct GetContestPageResult {
    pages: i32,
    contests: Vec<Contest>,
}

static SELECTOR_CONTESTS: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_TYPE: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_RATING_TYPE: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_NAME: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_DURATION: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_RATING_RANGE: OnceLock<Selector> = OnceLock::new();
static SELECTOR_CONTEST_PAGE_PAGINATION: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTESTS: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTEST_TYPE: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTEST_RATING_TYPE: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTEST_NAME: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTEST_DURATION: OnceLock<Selector> = OnceLock::new();
static SELECTOR2_CONTEST_RATING_RANGE: OnceLock<Selector> = OnceLock::new();

async fn get_contest_page() -> GetContestPageResult {
    log::info!("Get Contests Page");
    let mut contests: Vec<Contest> = vec![];

    let contest_page_html = reqwest::get("https://atcoder.jp/contests/?lang=ja").await.unwrap().text().await.unwrap();

    let document = scraper::Html::parse_document(&contest_page_html);

    let contests_time_selector = SELECTOR2_CONTESTS.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(1) time").unwrap());
    let contests_time = document.select(contests_time_selector);
    for element in contests_time {
        contests.push(Contest {
            start_time: element.inner_html(),
            ..Default::default()
        });
    }

    let contests_type_selector =
        SELECTOR2_CONTEST_TYPE.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(2) span:nth-child(1)").unwrap());
    let contest_type = document.select(contests_type_selector);
    for (i, element) in contest_type.enumerate() {
        let is_algo = element.inner_html() == "Ⓐ";
        if is_algo {
            contests[i].contest_type = ContestType::Algorithm;
        } else {
            contests[i].contest_type = ContestType::Heuristic;
        }
    }

    let contests_rating_type_selector =
        SELECTOR2_CONTEST_RATING_TYPE.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(2) span:nth-child(2)").unwrap());
    let contest_rating_type = document.select(contests_rating_type_selector);
    for (i, element) in contest_rating_type.enumerate() {
        let contest_rating_type = element.attr("class").unwrap();
        if contest_rating_type == "user-blue" {
            contests[i].contest_rating_type = ContestRatingType::ABC;
        } else if contest_rating_type == "user-orange" {
            contests[i].contest_rating_type = ContestRatingType::ARC;
        } else if contest_rating_type == "user-red" {
            contests[i].contest_rating_type = ContestRatingType::AGC;
        } else {
            contests[i].contest_rating_type = ContestRatingType::None;
        }
    }

    let contest_name_selector = SELECTOR2_CONTEST_NAME.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(2) a").unwrap());
    let contest_name = document.select(contest_name_selector);
    for (i, element) in contest_name.enumerate() {
        contests[i].contest_name = element.inner_html();
        let contest_link = element.attr("href").unwrap();
        let contest_id = &contest_link[10..];
        contests[i].contest_id = format!("{}.contest.atcoder.jp", contest_id);
        contests[i].url = contest_link.to_string();
    }

    let contest_duration_selector = SELECTOR2_CONTEST_DURATION.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(3)").unwrap());
    let contest_duration = document.select(contest_duration_selector);
    for (i, element) in contest_duration.enumerate() {
        let contest_duration_raw = element.inner_html();
        let contest_duration_split: Vec<&str> = contest_duration_raw.split(':').collect();
        let hour = contest_duration_split[0].parse::<i32>().unwrap();
        let minute = contest_duration_split[1].parse::<i32>().unwrap();
        contests[i].contest_duration = hour * 60 + minute;
    }

    let contest_rating_range_selector =
        SELECTOR2_CONTEST_RATING_RANGE.get_or_init(|| Selector::parse("div#contest-table-upcoming tbody tr td:nth-child(4)").unwrap());
    let contest_rating_range = document.select(contest_rating_range_selector);
    for (i, element) in contest_rating_range.enumerate() {
        let contest_rating_range_raw = element.inner_html();
        if contest_rating_range_raw == "All" {
            contests[i].rating_ragnge = (-998244353, 998244353);
        } else if contest_rating_range_raw == "-" {
            contests[i].rating_ragnge = (-998244353, -998244353);
        } else {
            let contest_rating_range_split: Vec<&str> = contest_rating_range_raw.split('~').collect();
            let rating_first_str = contest_rating_range_split[0].trim();
            let rating_end_str = contest_rating_range_split[1].trim();

            let rating_first = if rating_first_str.is_empty() {
                -998244353
            } else {
                rating_first_str.parse::<i32>().unwrap()
            };
            let rating_end = if rating_end_str.is_empty() {
                998244353
            } else {
                rating_end_str.parse::<i32>().unwrap()
            };

            contests[i].rating_ragnge = (rating_first, rating_end);
        }
        contests[i].rating_range_raw = contest_rating_range_raw;
    }

    GetContestPageResult { pages: 0, contests }
}

async fn get_past_contest_page(page: i32, get_pages: bool) -> GetContestPageResult {
    log::info!("Get Contest Page: page={}", page);

    let mut contests: Vec<Contest> = vec![];

    let contest_page_html = reqwest::get(format!("https://atcoder.jp/contests/archive?page={}&lang=ja", page)).await.unwrap().text().await.unwrap();

    let document = scraper::Html::parse_document(&contest_page_html);

    let contests_time_selector = SELECTOR_CONTESTS.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(1) time").unwrap());
    let contests_time = document.select(contests_time_selector);
    for element in contests_time {
        contests.push(Contest {
            start_time: element.inner_html(),
            ..Default::default()
        });
    }

    let contests_type_selector =
        SELECTOR_CONTEST_TYPE.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(2) span:nth-child(1)").unwrap());
    let contest_type = document.select(contests_type_selector);
    for (i, element) in contest_type.enumerate() {
        let is_algo = element.inner_html() == "Ⓐ";
        if is_algo {
            contests[i].contest_type = ContestType::Algorithm;
        } else {
            contests[i].contest_type = ContestType::Heuristic;
        }
    }

    let contests_rating_type_selector =
        SELECTOR_CONTEST_RATING_TYPE.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(2) span:nth-child(2)").unwrap());
    let contest_rating_type = document.select(contests_rating_type_selector);
    for (i, element) in contest_rating_type.enumerate() {
        let contest_rating_type = element.attr("class").unwrap();
        if contest_rating_type == "user-blue" {
            contests[i].contest_rating_type = ContestRatingType::ABC;
        } else if contest_rating_type == "user-orange" {
            contests[i].contest_rating_type = ContestRatingType::ARC;
        } else if contest_rating_type == "user-red" {
            contests[i].contest_rating_type = ContestRatingType::AGC;
        } else {
            contests[i].contest_rating_type = ContestRatingType::None;
        }
    }

    let contest_name_selector = SELECTOR_CONTEST_NAME.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(2) a").unwrap());
    let contest_name = document.select(contest_name_selector);
    for (i, element) in contest_name.enumerate() {
        contests[i].contest_name = element.inner_html();
        let contest_link = element.attr("href").unwrap();
        let contest_id = &contest_link[10..];
        contests[i].contest_id = format!("{}.contest.atcoder.jp", contest_id);
        contests[i].url = contest_link.to_string();
    }

    let contest_duration_selector = SELECTOR_CONTEST_DURATION.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(3)").unwrap());
    let contest_duration = document.select(contest_duration_selector);
    for (i, element) in contest_duration.enumerate() {
        let contest_duration_raw = element.inner_html();
        let contest_duration_split: Vec<&str> = contest_duration_raw.split(':').collect();
        let hour = contest_duration_split[0].parse::<i32>().unwrap();
        let minute = contest_duration_split[1].parse::<i32>().unwrap();
        contests[i].contest_duration = hour * 60 + minute;
    }

    let contest_rating_range_selector = SELECTOR_CONTEST_RATING_RANGE.get_or_init(|| Selector::parse("div.table-responsive tbody tr td:nth-child(4)").unwrap());
    let contest_rating_range = document.select(contest_rating_range_selector);
    for (i, element) in contest_rating_range.enumerate() {
        let contest_rating_range_raw = element.inner_html();
        if contest_rating_range_raw == "All" {
            contests[i].rating_ragnge = (-998244353, 998244353);
        } else if contest_rating_range_raw == "-" {
            contests[i].rating_ragnge = (-998244353, -998244353);
        } else {
            let contest_rating_range_split: Vec<&str> = contest_rating_range_raw.split('~').collect();
            let rating_first_str = contest_rating_range_split[0].trim();
            let rating_end_str = contest_rating_range_split[1].trim();

            let rating_first = if rating_first_str.is_empty() {
                -998244353
            } else {
                rating_first_str.parse::<i32>().unwrap()
            };
            let rating_end = if rating_end_str.is_empty() {
                998244353
            } else {
                rating_end_str.parse::<i32>().unwrap()
            };

            contests[i].rating_ragnge = (rating_first, rating_end);
        }
        contests[i].rating_range_raw = contest_rating_range_raw;
    }

    let mut max = -1;
    if get_pages {
        let contest_page_pagination_selector =
            SELECTOR_CONTEST_PAGE_PAGINATION.get_or_init(|| Selector::parse("a[href^=\"/contests/archive?lang=ja&\"]").unwrap());
        let contest_page_pagination = document.select(contest_page_pagination_selector);
        for element in contest_page_pagination {
            let query = Url::parse(format!("https://atcoder.jp{}", element.attr("href").unwrap()).as_str())
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == "page")
                .unwrap()
                .1
                .to_string()
                .parse::<i32>()
                .unwrap();
            max = std::cmp::max(max, query);
        }
    }

    let pages = max;

    GetContestPageResult { pages, contests }
}
pub async fn update_contests(pool: &Arc<Mutex<Pool>>) -> mysql::Result<()> {
    let first_page = get_contest_page().await;
    let mut contest_vec: Vec<Contest> = first_page.contests;
    sleep(Duration::from_millis(100)).await;
    let mut first_page = get_past_contest_page(1, true).await;
    contest_vec.append(&mut first_page.contests);
    sleep(Duration::from_millis(100)).await;
    log::info!("pages: {}", first_page.pages);
    for i in 2..=first_page.pages {
        let mut page = get_past_contest_page(i, false).await;
        contest_vec.append(&mut page.contests);
        sleep(Duration::from_millis(100)).await;
    }
    log::info!("Start Database Insert");
    let pool = pool.lock().await;
    let mut conn = pool.get_conn().unwrap();
    let contests: Vec<ContestDataTuple> = conn.query(r"SELECT `id`,  `contest_id`,  `name`,  `start_time`,  `duration`,  `url`,  `contest_type`,  `rating_type`,  `rating_range_start`,  `rating_range_end`,  `rating_range_raw`,  `get_user_ratings_flag` FROM contests").unwrap();
    let mut contests_list: HashSet<String> = HashSet::new();
    for i in contests {
        contests_list.insert(i.1);
    }
    for i in contest_vec {
        if !contests_list.contains(&i.contest_id) {
            log::info!("Insert to database: {}", &i.contest_id);
            conn.exec_drop(r"INSERT INTO contests (contest_id, name, start_time, duration, url, contest_type, rating_type, rating_range_start, rating_range_end, rating_range_raw, get_user_ratings_flag)
                            VALUES (:contest_id, :name, :start_time, :duration, :url, :type, :rating_type, :rating_range_start, :rating_range_end, :rating_range_raw, 0)",
                            params! {
                                "contest_id" => &i.contest_id,
                                "name" => i.contest_name,
                                "start_time" => i.start_time,
                                "duration" => i.contest_duration,
                                "url" => i.url,
                                "type" => match i.contest_type {
                                    ContestType::Algorithm => 0,
                                    ContestType::Heuristic => 1
                                },
                                "rating_type" => match i.contest_rating_type {
                                    ContestRatingType::ABC => 0,
                                    ContestRatingType::ARC => 1,
                                    ContestRatingType::AGC => 2,
                                    ContestRatingType::None => 3,
                                },
                                "rating_range_start" => i.rating_ragnge.0,
                                "rating_range_end" => i.rating_ragnge.1,
                                "rating_range_raw" => i.rating_range_raw
            }).unwrap();
        }
    }
    Ok(())
}
