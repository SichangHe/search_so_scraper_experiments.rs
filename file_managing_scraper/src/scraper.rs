use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use regex::Regex;
use reqwest::{Client, Response, Url};
use select::{document::Document, predicate::Name};
use tokio::time::sleep;

use crate::file_dealer::save_file;

pub struct CrawlerParallel {
    lw_clone: Arc<Mutex<VecDeque<Url>>>,
    ku_clone: Arc<Mutex<HashMap<Url, bool>>>,
    apc_clone: Arc<Mutex<usize>>,
    su_clone: Arc<Mutex<HashMap<Url, usize>>>,
    process_id: usize,
    pub processed_count: usize,
    idle: bool,
    client: Client,
    blacklist_re: Regex,
    whitelist_re: Regex,
    url: Url,
    final_url: Url,
}

impl CrawlerParallel {
    /// construct a new `CrawlerParallel`
    pub fn new(
        lw_clone: Arc<Mutex<VecDeque<Url>>>,
        ku_clone: Arc<Mutex<HashMap<Url, bool>>>,
        apc_clone: Arc<Mutex<usize>>,
        su_clone: Arc<Mutex<HashMap<Url, usize>>>,
        process_id: usize,
        blacklist_re: Regex,
        whitelist_re: Regex,
    ) -> Result<CrawlerParallel, Box<dyn Error>> {
        // initialize the HTTP client
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5)) // timeout: 5 sec
            .danger_accept_invalid_certs(true) // ignore certificate
            .build()?;

        let default_url = Url::parse("https://www.google.com/").unwrap(); // used as place holder

        Ok(CrawlerParallel {
            lw_clone,
            ku_clone,
            apc_clone,
            su_clone,
            process_id,
            processed_count: 0,
            idle: false,
            client,
            blacklist_re,
            whitelist_re,
            url: default_url.clone(),
            final_url: default_url,
        })
    }

    /// crawl URL from the waitlist\
    /// give back URL into it\
    /// save response as file
    pub async fn crawl(&mut self) {
        // increment active process count upon crawl start
        {
            // obtain the active_process_count lock and + 1
            let apc_lock = self.apc_clone.lock();
            let mut count = apc_lock.unwrap();
            *count += 1;
        }; // active_process_count unlock

        // repeatedly crawl
        loop {
            // check idle, break if no process running
            if self.idle_check().await {
                break;
            }

            // get a URL from waitlist, continue if waitlist empty
            if self.get_url().await {
                continue;
            }

            // process the URL
            if self.process_url().await {
                continue;
            }
        }
    }

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
                    println!("Process {} exits", self.process_id);
                    return true;
                } else {
                    println!(
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

    /// get a URL from waitlist\
    /// set self state to *idle* if empty waitlist\
    /// # return
    /// `false` normally\
    /// `true` if waitlist empty
    async fn get_url(&mut self) -> bool {
        // obtain the known_url lock
        let mut known_url = self.ku_clone.lock().unwrap();
        // obtain the link_waitlist lock
        let mut link_waitlist = self.lw_clone.lock().unwrap();

        // pop the first URL from waitlist until it was not checked
        loop {
            match link_waitlist.pop_front() {
                Some(url0) => {
                    if !*known_url.get_key_value(&url0).unwrap().1 {
                        // URL not checked
                        known_url.insert(url0.clone(), true);
                        self.url = url0;
                        return false;
                    }
                }

                None => {
                    // link_waitlist is empty
                    self.idle = true;
                    return true;
                }
            }
        }
    } // link_waitlist unlock

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
    async fn process_url(&mut self) -> bool {
        // make the request
        let response;
        // println!("    Process {}: Requesting {}", self.process_id, self.url); //DEBUG
        match self.client.get(self.url.clone()).send().await {
            Ok(r) => response = r,
            Err(err) => {
                println!(
                    "Process {} response: {} | {}",
                    self.process_id, err, self.url
                );
                return true;
            }
        }
        self.final_url = response.url().to_owned(); // URL after potential redirection

        // println!(
        //     "    Process {}: checking final_url {}",
        //     self.process_id, self.final_url
        // ); //DEBUG
        // check the final URL after potential redirection
        if self.final_url != self.url {
            if self.check_final_url().await {
                return true;
            }
        }

        // check response status
        if self.check_response_status(&response).await {
            return true;
        }

        // println!(
        //     "    Process {}: getting response header of {}",
        //     self.process_id, self.final_url
        // ); //DEBUG
        // get HTTP response headers
        let header;
        match self.get_headers(&response).await {
            None => return true,
            Some(h) => header = h,
        }

        // check file type: HTML or other
        if header.contains("text/html") {
            // type: HTML
            if self.process_html(response).await {
                return true;
            }
        } else {
            // type: other file
            //TODO: deal with type case by case
            if self.process_file(response).await {
                return true;
            }
        }

        self.processed_count += 1;

        false
    }

    /// check if `final_url` is already checked\
    /// record new URL as checked
    /// # return
    /// `false` for new and unfiltered URL\
    /// `true` for checked or filtered URL
    async fn check_final_url(&self) -> bool {
        let mut known_url = self.ku_clone.lock().unwrap();
        match known_url.get(&self.final_url) {
            Some(checked) => {
                // this URL is recorded
                if *checked {
                    //* this URL is already checked, skip
                    return true;
                } else {
                    // mark this URL as checked
                    known_url.insert(self.final_url.clone(), true);
                }
            }
            None => {
                // add this new URL as recorded and checked
                known_url.insert(self.final_url.clone(), true);
                // filter this newly found URL
                if self.blacklist_re.is_match(self.final_url.as_str()) {
                    // blacklist filtered, skip
                    return true;
                } else if !self.whitelist_re.is_match(self.final_url.as_str()) {
                    // whitelist filtered, skip
                    return true;
                }
            }
        }

        false
    } // known_url unlock

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

    /// # return
    /// "content-type" in HTTP response headers as `Some(header_str)`\
    /// or `None`
    async fn get_headers(&self, response: &Response) -> Option<String> {
        let header;
        match response.headers().get("content-type") {
            Some(h) => header = h,
            None => {
                println!(
                    "Process {} header: No header found | {}",
                    self.process_id, self.final_url
                );
                return None;
            }
        }
        let header_str;
        match header.to_str() {
            Ok(r) => header_str = r,
            Err(err) => {
                println!(
                    "Process {} header: {} | {}",
                    self.process_id, err, self.final_url
                );
                return None;
            }
        }

        Some(header_str.to_owned())
    }

    /// all the steps to process HTML\
    /// - get the text
    /// - find all the href and img src
    /// - store the links
    /// - save the HTML file
    /// # return
    /// `false` normally\
    /// `true` if anything failed
    async fn process_html(&self, response: Response) -> bool {
        let mut links = Vec::new();
        let html;
        // println!("    Process {}: awaiting text response", self.process_id); //DEBUG

        // get the text response
        match response.text().await {
            Ok(r) => html = r,
            Err(err) => {
                println!(
                    "Process {} text response: {} | {}",
                    self.process_id, err, self.final_url
                );
                return true;
            }
        }

        // iterate through all the href and img and store them in `links`
        {
            let document = Document::from(html.as_str());
            for href in document.find(Name("a")).filter_map(|n| n.attr("href")) {
                let href_url = self.final_url.join(href);
                match href_url {
                    Ok(href_url0) => links.push(href_url0),
                    Err(_err) => {
                        // println!("Process {} parse URL: {} | {}", self.process_id, err, href)
                        //DEBUG
                    }
                }
            }
            for img in document.find(Name("img")).filter_map(|n| n.attr("src")) {
                let img_url = self.final_url.join(img);
                match img_url {
                    Ok(img_url0) => links.push(img_url0),
                    Err(_err) => {
                        // println!("Process {} parse URL: {} | {}", self.process_id, err, img);
                        //DEBUG
                    }
                }
            }
            links.sort();
            links.dedup();
        }

        // println!(
        //     "    Process {}: processing {} links",
        //     self.process_id,
        //     links.len()
        // ); //DEBUG

        // check each link and add to known_url and link_waitlist
        self.process_links(links).await;

        // record URL as scraped
        let (scraped, index) = self.record_scraped().await;

        // save HTML
        if let Err(e) = save_file(scraped, &self.final_url, index, ".html", html.as_bytes()).await {
            println!(
                "Process {} save html: {} | {}",
                self.process_id, e, self.final_url
            );
            return true;
        }

        false
    }

    /// process and consume the links from HTML
    async fn process_links(&self, links: Vec<Url>) {
        // obtain the known_url lock
        let mut known_url = self.ku_clone.lock().unwrap();
        // obtain the link_waitlist lock
        let mut link_waitlist = self.lw_clone.lock().unwrap();

        for link in links {
            if let None = known_url.get(&link) {
                // URL not known
                // filter this newly found URL
                if self.blacklist_re.is_match(link.as_str()) {
                    // blacklist filtered, mark as checked
                    // println!("    Process {}: {} blacklisted", self.process_id, link); //DEBUG
                    known_url.insert(link, true);
                } else {
                    if !self.whitelist_re.is_match(link.as_str()) {
                        // whitelist filtered, mark as checked
                        // println!("    Process {}: {} not whitelisted", self.process_id, link); //DEBUG
                        known_url.insert(link, true);
                    } else {
                        // record and add to waitlist
                        link_waitlist.push_back(link.clone());
                        known_url.insert(link, false);
                    }
                }
            }
        }

        // println!(
        //     "    Process {}: {} items in waitlist",
        //     self.process_id,
        //     link_waitlist.len()
        // ); //DEBUG
    } // known_url unlock, link_waitlist unlock

    /// add the new URL to scraped_url
    /// # return
    /// if it has been scraped 'scraped'\
    /// its place in scraped_url as its newly obtained `index`
    pub async fn record_scraped(&self) -> (bool, usize) {
        // obtain the scraped_url lock
        let mut scraped_url = self.su_clone.lock().unwrap();

        if let Some(index) = scraped_url.get(&self.final_url) {
            return (true, *index); // URL scraped before
        }

        let index = scraped_url.len();
        scraped_url.insert(self.final_url.clone(), index);

        (false, index) // new URL
    } // scraped_url lock drop

    /// process the file
    /// - figure out its extension
    /// - save it
    /// # return
    /// `false` normally\
    /// `true` if something didn't go through
    /// - invalid extension
    /// - failed to receive response
    /// - failed to save
    async fn process_file(&self, response: Response) -> bool {
        // set file extension according to URL
        // last part separated by "." and first part separated by "?"
        let file_extension = match self
            .final_url
            .as_str()
            .split(".")
            .last()
            .unwrap_or_default()
            .split("?")
            .next()
        {
            None => {
                println!(
                    "Process {}: file format unknown | {}",
                    self.process_id, self.final_url
                );
                String::new()
            }
            Some(extension) => format!(".{}", extension),
        };

        // prevent crashing file system
        if file_extension.contains("/") {
            println!(
                "Process {}: invalid file format {} | {}",
                self.process_id, file_extension, self.final_url
            );
            return true;
        }

        // get the bytes of the file
        let bytes;
        // println!("    Process {}: awaiting bytes response", self.process_id); //DEBUG
        match response.bytes().await {
            Ok(r) => bytes = r,
            Err(err) => {
                println!(
                    "Process {} bytes response: {} | {}",
                    self.process_id, err, self.final_url
                );
                return true;
            }
        }

        // record URL as scraped
        let (scraped, index) = self.record_scraped().await;

        // save file
        if let Err(e) = save_file(scraped, &self.final_url, index, &file_extension, &bytes).await {
            println!(
                "Process {} save file: {} | {}",
                self.process_id, e, self.final_url
            );
            return true;
        }

        false
    }
}
