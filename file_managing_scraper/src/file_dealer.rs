use chrono::{Local, SecondsFormat::Secs};
use once_cell::sync::Lazy;
use reqwest::Url;
use std::{error::Error, path::Path};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

static TIME: Lazy<String> = Lazy::new(|| get_time());

pub async fn read_file(filename: &str) -> Result<String, Box<dyn Error>> {
    let mut f = File::open(filename).await?;
    let mut b = String::new();
    f.read_to_string(&mut b).await?;
    Ok(b)
}

pub async fn write_file(filename: &str, content: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(filename);
    create_dir_all(path.parent().unwrap()).await?;
    let mut f = File::create(filename).await?;
    f.write_all(&content.as_bytes()).await?;
    Ok(())
}

pub async fn write_file_bytes(filename: &str, content: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = Path::new(filename);
    create_dir_all(path.parent().unwrap()).await?;
    let mut f = File::create(filename).await?;
    f.write_all(content).await?;
    Ok(())
}

/// save to file under folder named after `index`\
/// named time and date
pub async fn save_file(
    scraped: bool,
    url: &Url,
    index: usize,
    file_extention: &str,
    content: &[u8],
) -> Result<(), String> {
    let filename = index.to_string() + "/" + &TIME + file_extention;
    for _ in 0..=2 {
        match write_file_bytes(&filename, content).await {
            Ok(()) => {
                if scraped {
                    return Ok(());
                }
            }
            Err(e) => {
                println!("{} saving {}", e, filename);
                continue;
            }
        }
        let url_txt = index.to_string() + "/url.txt";
        match write_file(&url_txt, url.as_str()).await {
            Ok(()) => return Ok(()),
            Err(e) => println!("{} writing url.txt for {}", e, filename),
        }
    }
    Err(format!("failed to save {}", filename))
}

fn get_time() -> String {
    let dt = Local::now();
    dt.to_rfc3339_opts(Secs, false)
}
