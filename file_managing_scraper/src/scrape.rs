use hhmmss::Hhmmss;
use regex::Regex;
use reqwest::Url;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use tokio::{spawn, time::Instant};

use crate::scraper::CrawlerParallel;

pub async fn scrape(
    process_num: usize,
    blacklist: Regex,
    whitelist: Regex,
    known_url: HashMap<Url, bool>,
    scraped_url: HashMap<Url, usize>,
) -> (HashMap<Url, bool>, HashMap<Url, usize>) {
    let start_time = Instant::now();

    let link_waitlist: VecDeque<Url> = scraped_url.keys().map(|u| Url::clone(u)).collect();
    let known_url = Arc::new(Mutex::new(known_url));
    let link_waitlist = Arc::new(Mutex::new(link_waitlist));
    let scraped_url = Arc::new(Mutex::new(scraped_url));

    // spawn `process_num` async processes
    let mut handles = Vec::new();
    let active_process_count = 0usize;
    let active_process_count = Arc::new(Mutex::new(active_process_count));

    for process_id in 0..process_num {
        let link_waitlist_clone = Arc::clone(&link_waitlist);
        let known_url_clone = Arc::clone(&known_url);
        let active_process_count_clone = Arc::clone(&active_process_count);
        let scraped_url_clone = Arc::clone(&scraped_url);
        let blacklist_clone = blacklist.clone();
        let whitelist_clone = whitelist.clone();

        handles.push(spawn(async move {
            // create a crawler and crawl
            let mut crawler = CrawlerParallel::new(
                link_waitlist_clone,
                known_url_clone,
                active_process_count_clone,
                scraped_url_clone,
                process_id,
                blacklist_clone,
                whitelist_clone,
            )
            .unwrap();
            crawler.crawl().await;

            crawler.processed_count // return the processed count
        }));
    }

    // count total URL processed successfully
    let mut total_processed_count = 0usize;
    for handle in handles {
        match handle.await {
            Ok(processed_count) => {
                total_processed_count += processed_count;
            }
            Err(err) => {
                println!("Master joining handle: {}", err)
            }
        }
    }

    let used_time = start_time.elapsed().hhmmssxxx();
    println!(
        "Finished scraping {} in {}",
        total_processed_count, used_time
    );

    let known_url = known_url.lock().unwrap().to_owned();
    let scraped_url = scraped_url.lock().unwrap().to_owned();

    (known_url, scraped_url)
}
