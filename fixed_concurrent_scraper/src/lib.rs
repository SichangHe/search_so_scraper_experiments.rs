use bytes::Bytes;
use regex::Regex;
use reqwest::{Client, Response};
use select::{document::Document, predicate::Name};
use sha256::digest;
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    spawn,
    time::{sleep, Duration},
};
use url::Url;

#[tokio::main]
pub async fn crawl_links_r(url0: Url, process_num: usize) -> Result<usize, Box<dyn Error>> {
    let mut known_urls = HashMap::new();
    let mut links_waitlist = VecDeque::new();

    links_waitlist.push_back(url0.clone());
    known_urls.insert(url0, false);

    let known_urls = Arc::new(Mutex::new(known_urls));
    let links_waitlist = Arc::new(Mutex::new(links_waitlist));

    // spawn `process_num` async processes
    let mut handles = Vec::new();
    let active_process_count = 0usize;
    let active_process_count = Arc::new(Mutex::new(active_process_count));

    {
        // the first process
        let links_waitlist_clone = Arc::clone(&links_waitlist);
        let known_urls_clone = Arc::clone(&known_urls);
        let active_process_count_clone = Arc::clone(&active_process_count);
        handles.push(spawn(async move {
            // create a crawler and crawl
            let mut crawler = CrawlerParallel::new(
                links_waitlist_clone,
                known_urls_clone,
                active_process_count_clone,
                0,
            )
            .unwrap();
            crawler.crawl().await;

            crawler.processed_count // return the processed count
        }));
    }

    sleep(Duration::from_secs(5)).await; // wait for process 0 to finish 1 round

    for process_id in 1..process_num {
        let links_waitlist_clone = Arc::clone(&links_waitlist);
        let known_urls_clone = Arc::clone(&known_urls);
        let active_process_count_clone = Arc::clone(&active_process_count);
        handles.push(spawn(async move {
            // create a crawler and crawl
            let mut crawler = CrawlerParallel::new(
                links_waitlist_clone,
                known_urls_clone,
                active_process_count_clone,
                process_id,
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
                eprintln!("Master joining handle: {}", err)
            }
        }
    }

    Ok(total_processed_count)
}

async fn save_file(path: &str, file_bytes: &Bytes) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(&path).await?;
    file.write_all(file_bytes).await?;
    Ok(())
}

async fn save_html(path: &str, html: &String) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path).await?;
    file.write_all(html.as_bytes()).await?;
    Ok(())
}

struct CrawlerParallel {
    lw_clone: Arc<Mutex<VecDeque<Url>>>,
    ku_clone: Arc<Mutex<HashMap<Url, bool>>>,
    apc_clone: Arc<Mutex<usize>>,
    process_id: usize,
    pub processed_count: usize,
    idle: bool,
    client: Client,
    blacklist_re: Regex,
    whitelist_re: Regex,
}

impl CrawlerParallel {
    /// construct a new `CrawlerParallel`
    fn new(
        lw_clone: Arc<Mutex<VecDeque<Url>>>,
        ku_clone: Arc<Mutex<HashMap<Url, bool>>>,
        apc_clone: Arc<Mutex<usize>>,
        process_id: usize,
    ) -> Result<CrawlerParallel, Box<dyn Error>> {
        // initialize the HTTP client
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5)) // timeout: 5 sec
            .danger_accept_invalid_certs(true) // ignore certificate
            .build()?;

        // regex filter
        let blacklist_re=Regex::new(r".*(@)|(about/about)|(/event-list[/?])|(/node)|(node_tid)|(print/)|(/recruiting-events[/?])|(/printpdf/)|(\d{4}-\d{2}\D*).*").unwrap();
        let whitelist_re = Regex::new(r".*[/\.]dukekunshan\.edu\.cn.*").unwrap();

        Ok(CrawlerParallel {
            lw_clone,
            ku_clone,
            apc_clone,
            process_id,
            processed_count: 0,
            idle: false,
            client,
            blacklist_re,
            whitelist_re,
        })
    }

    /// crawl URL from the waitlist\
    /// give back URL into it\
    /// save response as file
    async fn crawl(&mut self) {
        // increment active process count upon crawl start
        {
            // obtain the active_process_count lock and + 1
            let apc_lock = self.apc_clone.lock();
            let mut count = apc_lock.unwrap();
            *count += 1;
        }; // active_process_count unlock

        // repeatedly crawl
        loop {
            let url;

            // check idle, break if no process running
            if self.idle_check().await {
                break;
            }

            // get a URL from waitlist, continue if waitlist empty
            match self.get_url().await {
                Some(url0) => url = url0,
                None => continue,
            }

            // process the URL
            if self.process_url(url).await {
                continue;
            }
        }
    }

    /// process the URL given
    /// - HTTP request
    /// - check final URL after potential redirection
    /// - process the HTML or other file
    /// # return
    /// `false` normally\
    /// `true` if something didn't go through, including:
    /// - the URL is already checked
    /// - the response status is wrong
    /// - failed to get the HTTP response headers
    /// - something went wrong when processing the file
    async fn process_url(&mut self, url: Url) -> bool {
        // make the request
        let response;
        // eprintln!("    Process {}: Requesting {}", self.process_id, url); //DEBUG
        match self.client.get(url.clone()).send().await {
            Ok(r) => response = r,
            Err(err) => {
                eprintln!("Process {} response: {}", self.process_id, err);
                return true;
            }
        }
        let final_url = response.url().to_owned(); // URL after potential redirection

        // check the final URL after potential redirection
        if final_url != url {
            if self.check_final_url(final_url.clone()).await {
                return true;
            }
        }

        // check response status
        if self.check_response_status(&response).await {
            return true;
        }

        // get HTTP response headers
        let header;
        match self.get_headers(&response).await {
            None => return true,
            Some(h) => header = h,
        }

        // check file type: HTML or other
        if header.contains("text/html") {
            // type: HTML
            if self.process_html(response, url, final_url).await {
                return true;
            }
        } else {
            // type: other file
            //TODO: deal with type case by case
            if self.process_file(response, url, final_url).await {
                return true;
            }
        }

        self.processed_count += 1;

        false
    }

    /// process the file
    /// - figure out its extension
    /// - save it
    /// # return
    /// `false` normally\
    /// `true` if something didn't go through
    /// - invalid extension
    /// - failed to receive response
    /// - failed to save
    async fn process_file(&self, response: Response, url: Url, final_url: Url) -> bool {
        // set file extension according to URL
        // last part separated by "." and first part separated by "?"
        let file_extension = match url
            .as_str()
            .split(".")
            .last()
            .unwrap_or_default()
            .split("?")
            .next()
        {
            None => {
                eprintln!("Process {}: file format unknown | {}", self.process_id, url);
                String::new()
            }
            Some(extension) => format!(".{}", extension),
        };

        // prevent crashing file system
        if file_extension.contains("/") {
            eprintln!(
                "Process {}: invalid file format {}",
                self.process_id, file_extension
            );
            return true;
        }

        // get the bytes of the file
        let bytes;
        // eprintln!("    Process {}: awaiting bytes response", self.process_id); //DEBUG
        match response.bytes().await {
            Ok(r) => bytes = r,
            Err(err) => {
                eprintln!("Process {} bytes response: {}", self.process_id, err);
                return true;
            }
        }

        // save file
        let path = digest(final_url.as_str()) + &file_extension;
        if self.try_save_file(&path, bytes).await {
            // failed to save
            eprintln!("Process {} save file failed", self.process_id);
            return true;
        };

        // output `"final_url": "path"`
        println!("\"{}\": \"{}\"", final_url, path);
        false
    }

    /// all the steps to process HTML\
    /// - get the text
    /// - find all the href and img src
    /// - store the links
    /// - save the HTML file
    /// # return
    /// `false` normally\
    /// `true` if anything failed
    async fn process_html(&self, response: Response, url: Url, final_url: Url) -> bool {
        let mut links = Vec::new();
        let html;
        // eprintln!("    Process {}: awaiting text response", self.process_id); //DEBUG

        // get the text response
        match response.text().await {
            Ok(r) => html = r,
            Err(err) => {
                eprintln!("Process {} text response: {}", self.process_id, err);
                return true;
            }
        }

        // iterate through all the href and img and store them in `links`
        {
            let document = Document::from(html.as_str());
            for href in document.find(Name("a")).filter_map(|n| n.attr("href")) {
                let href_url = final_url.join(href);
                match href_url {
                    Ok(href_url0) => links.push(href_url0),
                    Err(_err) => {
                        // eprintln!("Process {} parse url: {} | {}", self.process_id, err, href) //DEBUG
                    }
                }
            }
            for img in document.find(Name("img")).filter_map(|n| n.attr("src")) {
                let img_url = final_url.join(img);
                match img_url {
                    Ok(img_url0) => links.push(img_url0),
                    Err(_err) => {
                        // eprintln!("Process {} parse url: {} | {}", self.process_id, err, img); //DEBUG
                    }
                }
            }
            links.sort();
            links.dedup();
        }

        // check each link and add to known_urls and links_waitlist
        self.process_links(links).await;

        // save HTML
        let path = digest(final_url.as_str()) + ".html";
        if self.try_save_html(&path, html).await {
            // failed to save
            eprintln!("Process {} save HTML failed | {}", self.process_id, url);
            return true;
        };

        // output `"final_url": "path"`
        println!("\"{}\": \"{}\"", final_url, path);
        false
    }

    /// try to save given HTML to given path for 3 times
    /// # return
    /// `true` if failed\
    /// `false` if successful
    async fn try_save_html(&self, path: &str, html: String) -> bool {
        for _ in 0..3 {
            // try saving file 3 times, sleep 5 sec if fail each time
            match save_html(&path, &html).await {
                Err(err) => {
                    eprintln!("Process {} save HTML: {}", self.process_id, err);
                }
                _ => {
                    // eprintln!("Process {} html saved", self.process_id);
                    return false;
                }
            };
            // eprintln!("Process {}: awaiting file fail sleep", self.process_id); //DEBUG

            sleep(Duration::from_secs(5)).await;
        }

        true
    }

    /// try to save given file to given path for 3 times
    /// # return
    /// `true` if failed\
    /// `false` if successful
    async fn try_save_file(&self, path: &str, bytes: Bytes) -> bool {
        for _ in 0..3 {
            // try saving file 3 times, sleep 5 sec if fail each time
            match save_file(&path, &bytes).await {
                Err(err) => {
                    eprintln!("Process {} save file: {}", self.process_id, err);
                }
                _ => {
                    // eprintln!("Process {} html saved", self.process_id);
                    return false;
                }
            };
            // eprintln!("Process {}: awaiting file fail sleep", self.process_id); //DEBUG

            sleep(Duration::from_secs(5)).await;
        }

        true
    }

    /// process and consume the links from HTML
    async fn process_links(&self, links: Vec<Url>) {
        // wrap in scope so lock drop
        // obtain the known_url lock
        let ku_lock = self.ku_clone.lock();
        // obtain the links_waitlist lock
        let lw_lock = self.lw_clone.lock();

        let mut known_urls = ku_lock.unwrap();
        let mut links_waitlist = lw_lock.unwrap();

        for link in links {
            if let None = known_urls.get_key_value(&link) {
                // url not known
                // filter this newly found URL
                if self.blacklist_re.is_match(link.as_str()) {
                    // blacklist filtered, mark as checked
                    known_urls.insert(link, true);
                } else {
                    if !self.whitelist_re.is_match(link.as_str()) {
                        // whitelist filtered, mark as checked
                        known_urls.insert(link, true);
                    } else {
                        // record and add to waitlist
                        links_waitlist.push_back(link.clone());
                        known_urls.insert(link, false);
                    }
                }
            }
        }
    } // known_urls unlock, link_waitlist unlock

    /// # return
    /// "content-type" in HTTP response headers as `Some(header_str)`\
    /// or `None`
    async fn get_headers(&self, response: &Response) -> Option<String> {
        let header;
        match response.headers().get("content-type") {
            Some(h) => header = h,
            None => {
                eprintln!("Process {} header: No header found", self.process_id);
                return None;
            }
        }
        let header_str;
        match header.to_str() {
            Ok(r) => header_str = r,
            Err(err) => {
                eprintln!("Process {} header: {}", self.process_id, err);
                return None;
            }
        }

        Some(header_str.to_owned())
    }

    /// check status of response
    /// # return
    /// `false` normally\
    /// `true` if the status is within 400-599
    async fn check_response_status(&self, response: &Response) -> bool {
        let status = response.status();

        if status.is_client_error() {
            return true;
        }
        if status.is_server_error() {
            return true;
        }

        false
    }

    /// check if `final_url` is already checked\
    /// record new URL as checked
    /// # return
    /// `false` for new and unfiltered URL\
    /// `true` for checked or filtered URL
    async fn check_final_url(&self, final_url: Url) -> bool {
        // obtain the known_url lock
        let ku_lock = self.ku_clone.lock();
        let mut known_urls = ku_lock.unwrap();

        match known_urls.get_key_value(&final_url) {
            Some((_, checked)) => {
                // this URL is recorded
                if *checked {
                    //* this url is already checked, skip
                    return true;
                } else {
                    // mark this url as checked
                    known_urls.insert(final_url.clone(), true);
                }
            }
            None => {
                // add this new url as recorded and checked
                known_urls.insert(final_url.clone(), true);
                // filter this newly found URL
                if self.blacklist_re.is_match(final_url.as_str()) {
                    // blacklist filtered, skip
                    return true;
                } else if !self.whitelist_re.is_match(final_url.as_str()) {
                    // whitelist filtered, skip
                    return true;
                }
            }
        }

        false
    } // known_urls unlock

    /// get a URL from waitlist\
    /// set self state to *idle* if empty waitlist\
    /// # return
    /// `Some(url)` normally\
    /// `None` if waitlist empty
    async fn get_url(&mut self) -> Option<Url> {
        // obtain the known_url lock
        let ku_lock = self.ku_clone.lock();
        // obtain the links_waitlist lock
        let lw_lock = self.lw_clone.lock();

        let mut known_urls = ku_lock.unwrap();
        let mut links_waitlist = lw_lock.unwrap();

        // pop the first URL from waitlist until it was not checked
        loop {
            match links_waitlist.pop_front() {
                Some(url0) => {
                    if !*known_urls.get_key_value(&url0).unwrap().1 {
                        // URL not checked
                        known_urls.insert(url0.clone(), true);
                        return Some(url0);
                    }
                }

                None => {
                    // link_waitlist is empty
                    self.idle = true;
                    return None;
                }
            }
        }
    } // links_waitlist unlock

    /// check if idle\
    /// if idle, decrement `apc`, sleep 5 sec, increment `apc`
    /// # return
    /// `false` normally\
    /// `true` if 0 process running after decrement
    async fn idle_check(&mut self) -> bool {
        if self.idle {
            // idle, decrement active process count, sleep, increment
            {
                // obtain the active_process_count lock and - 1
                let apc_lock = self.apc_clone.lock();
                let mut count = apc_lock.unwrap();
                *count -= 1;
                // send exit signal if no process running
                if *count == 0 {
                    eprintln!("Process {} exits", self.process_id);
                    return true;
                } else {
                    eprintln!(
                        "Process {}: in idle with {} processes running",
                        self.process_id, count
                    );
                }
            }; // active_process_count unlock

            sleep(Duration::from_secs(5)).await; // sleep 5 sec
            self.idle = false;

            {
                // obtain the active_process_count lock and + 1
                let apc_lock = self.apc_clone.lock();
                let mut count = apc_lock.unwrap();
                *count += 1;
            } // active_process_count unlock
        }

        false
    }
}
