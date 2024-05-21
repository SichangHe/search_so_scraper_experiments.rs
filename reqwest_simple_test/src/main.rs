use bytes::Bytes;
use image;
use reqwest::{redirect::Policy, Client};
use std::{error::Error, process::exit, time::Duration};

fn main() {
    let url1 = "https://archlinux.org";
    let body = request(url1).unwrap_or_else(|err| {
        eprintln!("{}", err);
        String::from("nothing")
    });
    println!("{}", body);

    let url2 = "https://archlinux.org/static/shells_logo.a9dc284565e5.png";
    let body2 = request_as_bytes(url2).unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit(1);
    });
    let img = image::load_from_memory(&body2).unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit(1);
    });
    img.save("sample.jpg").unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}

#[tokio::main]
async fn request(url: &str) -> Result<String, Box<dyn Error>> {
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
async fn request_as_bytes(url: &str) -> Result<Bytes, Box<dyn Error>> {
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
