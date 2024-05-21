use crate::file_dealer::write_file;
use reqwest::Url;
use std::collections::{BTreeMap, HashMap};

pub async fn write_known_url(known_url: HashMap<Url, bool>) {
    let mut s = String::new();

    let mut sorted: Vec<String> = known_url.into_keys().map(|u| u.to_string()).collect();
    sorted.sort();

    for url in sorted {
        s.push_str(&url);
        s.push('\n');
    }
    loop {
        match write_file("known_url.txt", &s).await {
            Ok(()) => break,
            Err(e) => println!("{} saving known url", e),
        }
    }
}

pub async fn write_scraped_url(scraped_url: HashMap<Url, usize>) {
    let mut s = String::new();

    let scraped_url: BTreeMap<String, usize> = scraped_url
        .into_iter()
        .map(|(u, i)| (u.to_string(), i))
        .collect();

    for (url, index) in scraped_url {
        s.push_str(&url);
        s.push('<');
        s.push_str(&index.to_string());
        s.push('\n');
    }
    loop {
        match write_file("scraped_url.txt", &s).await {
            Ok(()) => break,
            Err(e) => println!("{} saving known url", e),
        }
    }
}
