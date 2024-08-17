extern crate mysql;

mod commands;
mod init;
mod scraping;
mod send_message;
mod utils;
mod web_server;

use init::init_logger;

use chrono::Timelike;
use poise::serenity_prelude::ActivityData;
use reqwest::cookie::Jar;
use scraping::atcoder_ratings::get_ratings;
use scraping::contests::update_contests;
use scraping::get_ranking::get_ranking;
use scraping::get_submission::get_submission;
use scraping::get_user_list;
use sha2::Digest;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use web_server::start;

use commands::atcoder;
use commands::server;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use scraping::login;

use mysql::*;
use tokio::time::sleep;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
pub struct Data {
    conn: Arc<Mutex<Pool>>,
    avatar_url: String,
}

async fn interval(ctx: serenity::Context) {
    log::info!("interval");
    let cookie_store = Arc::new(Jar::default());
    let mut last_minute = 100;
    let mut date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let url = format!(
        "mysql://{}:{}@localhost:3306/atcoder_notify",
        std::env::var("MYSQL_USER").expect(""),
        std::env::var("MYSQL_PASS").expect("")
    );
    let pool_raw = Pool::new(url.as_str()).unwrap();
    let pool = Arc::new(Mutex::new(pool_raw.clone()));
    let pool_clone = Arc::new(Mutex::new(pool_raw.clone()));
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        get_submission(&pool_clone, &ctx_clone).await;
    });
    loop {
        let now = chrono::Local::now();
        if date != now.date_naive() {
            log::info!("日ごとの処理開始");
            login::login(
                std::env::var("ATCODER_USER").expect(""),
                std::env::var("ATCODER_PASS").expect(""),
                &cookie_store,
            );
            update_contests(&pool).await;
            send_message::send_notify(&pool, &ctx).await;
            get_user_list::user_list_update(&pool).await;
            log::info!("日ごとの処理終了");
            date = now.date_naive();
        } else if last_minute != now.minute() {
            log::info!("分ごとの処理");
            get_ranking(&pool, &cookie_store, &ctx).await;
            get_ratings(&cookie_store, &pool, false).await;
            log::info!("分ごとの処理終了");
            last_minute = now.minute();
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    init_logger();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::all();
    let url = format!(
        "mysql://{}:{}@localhost:3306/atcoder_notify",
        std::env::var("MYSQL_USER").expect(""),
        std::env::var("MYSQL_PASS").expect("")
    );
    let pool = Pool::new(url.as_str()).unwrap();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![atcoder::atcoder(), server::server()],
            on_error: |error| {
                Box::pin(async move {
                    error
                        .ctx()
                        .unwrap()
                        .say("エラーが発生しました。")
                        .await
                        .unwrap();
                })
            },
            event_handler: |ctx, event, _framework, _data| {
                Box::pin(async move {
                    if let serenity::FullEvent::Message {
                        new_message: message,
                    } = event
                    {
                        println!("{}", message.content);
                        if format!("{:x}", sha2::Sha256::digest(&message.content))
                            == "a69893e03d93e1e4d0f66dd41e9df574b70d8f3281ef499eaf04e0437d3cad17"
                        {
                            message
                                .reply(
                                    &ctx.http,
                                    std::env::var("SEACRET_COMMAND_OUTPUT")
                                        .unwrap_or("** **".to_string()),
                                )
                                .await
                                .unwrap_or_default();
                        }
                    }
                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                thread::spawn(start);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tokio::spawn(interval(ctx.clone()));
                log::info!("Bot started as \"{}\"", ready.user.name);
                ctx.set_activity(Option::from(ActivityData::playing("元気にAtCoderを監視中")));
                Ok(Data {
                    conn: Arc::new(Mutex::new(pool)),
                    avatar_url: ready.user.avatar_url().unwrap(),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
