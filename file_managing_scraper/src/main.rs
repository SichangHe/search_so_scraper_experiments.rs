use file_managing_scraper::{
    get_existing::{get_blacklist, get_known_url, get_scraped_url, get_whitelist},
    scrape::scrape,
    write_new::{write_known_url, write_scraped_url},
};
use tokio::spawn;

#[tokio::main]
async fn main() {
    let process_num: usize;
    let blacklist;
    let whitelist;
    let known_url;
    let scraped_url;

    // get existing data
    {
        let process_num_handle = spawn(async {
            let mut args = std::env::args();
            args.next();
            args.next().unwrap().parse().unwrap()
        }); // get args[1]: number of process
        let blacklist_handle = spawn(async { get_blacklist().await });
        let whitelist_handle = spawn(async { get_whitelist().await });
        let known_url_handle = spawn(async { get_known_url().await });
        let scraped_url_handle = spawn(async { get_scraped_url().await });

        process_num = process_num_handle.await.unwrap();
        blacklist = blacklist_handle.await.unwrap();
        whitelist = whitelist_handle.await.unwrap();
        known_url = known_url_handle.await.unwrap();
        scraped_url = scraped_url_handle.await.unwrap();
    }

    // println!("blacklist:\n{}\n\nwhitelist:\n{}", blacklist, whitelist); //DEBUG

    // scrape new data
    let (known_url, scraped_url) =
        scrape(process_num, blacklist, whitelist, known_url, scraped_url).await;

    // write new data
    let write_new_url_handle = spawn(async move {
        write_known_url(known_url).await;
    });
    let write_scraped_url_handle = spawn(async move {
        write_scraped_url(scraped_url).await;
    });

    write_new_url_handle.await.unwrap();
    write_scraped_url_handle.await.unwrap();
}
