use selfcontained_scraper::crawl_links_r;
use url::Url;

#[tokio::main]
async fn main() {
    // let url = Url::parse("https://dukekunshan.edu.cn").unwrap();
    // let url = Url::parse("http://mp.weixin.qq.com/mp/homepage?__biz=MzU3ODg4NTQxMg==&hid=1&sn=97027463b2605823ee4aefdea01efaf9&scene=18#wechat_redirect").unwrap();

    let mut args = std::env::args();
    args.next();
    let url_string = args.next().unwrap();
    let url = Url::parse(&url_string).unwrap();

    let handle = tokio::task::spawn_blocking(move || crawl_links_r(url).unwrap());
    let _all_urls_wrap = handle.await;
    eprintln!("\n\nEnd");
}
