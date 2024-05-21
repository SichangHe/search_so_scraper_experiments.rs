use crate::file_dealer::read_file;
use regex::Regex;
use reqwest::Url;
use std::collections::HashMap;

/// get blacklist from blacklist.txt
pub async fn get_blacklist() -> Regex {
    let r;
    loop {
        match read_file("blacklist.txt").await {
            Ok(re) => {
                r = re;
                break;
            }
            Err(e) => println!("{} getting blacklist", e),
        }
    }
    Regex::new(&r).unwrap()
}

/// get whitelist from whitelist.txt
pub async fn get_whitelist() -> Regex {
    let r;
    loop {
        match read_file("whitelist.txt").await {
            Ok(re) => {
                r = re;
                break;
            }
            Err(e) => println!("{} getting whitelist", e),
        }
    }
    Regex::new(&r).unwrap()
}

/// get the map of known URL from file `known_url.txt`\
/// the file must contain `URL` each line\
/// the URL must be valid\
/// all bool `checked` initialized as `false`
pub async fn get_known_url() -> HashMap<Url, bool> {
    let s;
    loop {
        match read_file("known_url.txt").await {
            Ok(str) => {
                s = str;
                break;
            }
            Err(e) => println!("{} getting known_url", e),
        }
    }

    let mut m = HashMap::new();
    for line in s.split_whitespace() {
        if let Ok(u) = Url::parse(line) {
            m.insert(u, false); // push URL
        }
    }

    m
}

/// get the map of scraped URL from file `scraped_url.txt`\
/// the file must contain `url<index` each line\
/// the URL must be valid; `index` ≥ 0
pub async fn get_scraped_url() -> HashMap<Url, usize> {
    let s;
    loop {
        match read_file("scraped_url.txt").await {
            Ok(str) => {
                s = str;
                break;
            }
            Err(e) => println!("{} getting scraped_url", e),
        }
    }

    let mut m = HashMap::new();
    for line in s.split_whitespace() {
        let mut ele = line.split("<");
        let url;
        match ele.next() {
            Some(u) => url = Url::parse(u).unwrap(),
            None => continue,
        }
        let index: usize = ele.next().unwrap().parse().unwrap();
        m.insert(url, index);
    }

    m
}
