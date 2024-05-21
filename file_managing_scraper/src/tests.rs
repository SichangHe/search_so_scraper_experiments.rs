use std::collections::HashMap;

use reqwest::Url;

use crate::{file_dealer::write_file_bytes, get_existing::*, write_new::*};

#[tokio::main]
#[test]
async fn test_get_existing() {
    println!(
        "blacklist: {}\nwhitelist: {}\nknown_url: {:?}\nscraped_url: {:?}",
        get_blacklist().await,
        get_whitelist().await,
        get_known_url().await,
        get_scraped_url().await
    )
}

#[tokio::main]
#[test]
async fn test_write_new() {
    let mut known_url = HashMap::new();
    known_url.insert(Url::parse("https://dukekunshan.edu.cn/contact-us").unwrap(), true);
    known_url.insert(Url::parse("https://dukekunshan.edu.cn/").unwrap(), true);
    known_url.insert(
        Url::parse("https://dukekunshan.edu.cn/about").unwrap(),
        false,
    );
    write_known_url(known_url).await;

    let mut scraped_url = HashMap::new();
    scraped_url.insert(Url::parse("https://dukekunshan.edu.cn/contact-us").unwrap(), 2);
    scraped_url.insert(Url::parse("https://dukekunshan.edu.cn/").unwrap(), 0);
    scraped_url.insert(Url::parse("https://dukekunshan.edu.cn/about").unwrap(), 1);
    write_scraped_url(scraped_url).await;
}

#[tokio::main]
#[test]
async fn test_write_bytes() {
    write_file_bytes("001/test.txt", String::from("test").as_bytes())
        .await
        .unwrap_or_else(|e| println!("{}", e))
}

#[tokio::main]
#[test]
async fn test_read_new() {
    let known_url = get_known_url().await;
    println!("known_url:\n{:?}", known_url);

    let scraped_url = get_scraped_url().await;
    println!("scraped_url:\n{:?}", scraped_url);
}
