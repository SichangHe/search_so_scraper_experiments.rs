use bytes::Bytes;
use reqwest::{redirect::Policy, Client};
use select::{document::Document, predicate::Name};
use sha256::digest_bytes;
use std::{error::Error, fs::File, io::Write, time::Duration};
use url::Url;

#[tokio::main]
pub async fn save_all_image(url: &str) -> Result<String, Box<dyn Error>> {
    eprintln!("Scanning for {}", url);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5)) // timeout: 5 sec
        .redirect(Policy::none())
        .build()?;
    let body = client.get(url).send().await?.text().await?;
    let mut record = String::new();
    for img in Document::from(body.as_str())
        .find(Name("img"))
        .filter_map(|n| n.attr("src"))
    {
        let img_url = String::from(Url::parse(url)?.join(img)?.as_str());
        let img_format = match img.split(".").last() {
            None => {
                eprintln!("file format unknown");
                String::new()
            }
            Some(format) => format!(".{}", format),
        };
        eprintln!("img = {}", &img_url);
        let img_bytes = client.get(&img_url).send().await?.bytes().await?;
        record.push_str(
            format!(
                "{{{}}}:{{{}}}\n",
                img_url,
                save_image(img_format, &img_bytes)?
            )
            .as_str(),
        );
        // record.push_str(format!("{{{}}}:{{{}}}\n", img_url, save_image(&img_url)?).as_str());
    }
    Ok(record)
}

pub fn save_image(img_format: String, img_bytes: &Bytes) -> Result<String, Box<dyn Error>> {
    let hash = digest_bytes(img_bytes);
    let path = hash.clone() + &img_format;
    let mut file = File::create(&path)?;
    file.write_all(img_bytes)?;
    Ok(path)
}
