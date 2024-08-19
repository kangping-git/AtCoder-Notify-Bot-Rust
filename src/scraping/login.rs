use scraper::Selector;

use reqwest::{blocking::Client, cookie::Jar};
use std::sync::Arc;

pub fn login(user: String, pass: String, cookie_store: &Arc<Jar>) {
    log::info!("ログイン処理開始");

    let client = Client::builder().cookie_store(true).cookie_provider(Arc::clone(cookie_store)).build().unwrap();

    let selector = Selector::parse("input[name=\"csrf_token\"]").unwrap();
    let body = client.get("https://atcoder.jp/login").send().unwrap().text().unwrap();
    let document = scraper::Html::parse_document(&body);
    let csrf_token = document.select(&selector).next().unwrap().value().attr("value").unwrap().to_string();

    log::info!("csrf_token取得完了");

    let params = [("username", &user), ("password", &pass), ("csrf_token", &csrf_token)];

    let response = client.post("https://atcoder.jp/login?continue=https%3A%2F%2Fatcoder.jp%2Fhome").query(&params).send().unwrap();

    log::info!("レスポンス取得完了");

    if response.url().to_string() != "https://atcoder.jp/home" {
        log::warn!("ログインに失敗しました");
    }
}
