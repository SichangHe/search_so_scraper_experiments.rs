use bytes::Bytes;
use image;
use reqwest::{redirect::Policy, Client};
use select::{document::Document, predicate::Name};
use sha256::digest_bytes;
use std::{error::Error, time::Duration};
use url::Url;

#[tokio::main]
pub async fn save_all_image(url: &str) -> Result<String, Box<dyn Error>> {
    eprintln!("Scanning for {}", url);
    // let body = request(url)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .redirect(Policy::none())
        .build()?;
    let body = client.get(url).send().await?.text().await?;
    let mut record = String::new();
    for img in Document::from(body.as_str())
        .find(Name("img"))
        .filter_map(|n| n.attr("src"))
    {
        let img_url = String::from(Url::parse(url)?.join(img)?.as_str());
        println!("img = {}", &img_url);
        let img_bytes = client.get(&img_url).send().await?.bytes().await?;
        record.push_str(format!("{{{}}}:{{{}}}\n", img_url, save_image(&img_bytes)?).as_str());
        // record.push_str(format!("{{{}}}:{{{}}}\n", img_url, save_image(&img_url)?).as_str());
    }
    Ok(record)
}

pub fn save_image(img_bytes: &Bytes) -> Result<String, Box<dyn Error>> {
    // let img_bytes = request_as_bytes(url)?;
    match image::load_from_memory(img_bytes) {
        Err(err) => {
            eprintln!("{}", err);
            Ok(String::new())
        }
        Ok(img) => {
            let hash = digest_bytes(img_bytes);
            img.save(format!("{}.jpg", hash))?;
            Ok(hash)
        }
    }
}

#[tokio::main]
pub async fn request(url: &str) -> Result<String, Box<dyn Error>> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .redirect(Policy::none())
        .build()?
        .get(url)
        .send()
        .await?
        .text()
        .await?)
}

#[tokio::main]
pub async fn request_as_bytes(url: &str) -> Result<Bytes, Box<dyn Error>> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .redirect(Policy::none())
        .build()?
        .get(url)
        .send()
        .await?
        .bytes()
        .await?)
}
