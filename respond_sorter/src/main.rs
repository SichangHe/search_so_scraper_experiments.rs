use reqwest::{redirect::Policy, Client};
use std::{error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5)) // timeout: 5 sec
        .redirect(Policy::none())
        .build()?;

    let url0 = "https://dukekunshan.edu.cn";
    let response0 = client.get(url0).send().await?;
    let header0 = response0.headers();
    println!("0:{:?}", header0);

    let url1 = "https://dukekunshan.edu.cn/sites/all/themes/kunshan_new/images/logo-header.png";
    let response1 = client.get(url1).send().await?;
    let header1 = response1.headers();
    println!("1:{:?}", header1);

    // let url2 = "advising@dukekunshan.edu.cn";
    // let response2 = client.get(url2).send().await?;
    // let header2 = response2.headers();
    // println!("2:{:?}", header2);

    let url3 = "https://static.dukekunshan.edu.cn/sites/all/modules/colorbox_node/colorbox_node.css?r6b83q";
    let response3 = client.get(url3).send().await?;
    let header3 = response3.headers();
    println!("3:{:?}", header3);

    let url4 = "https://static.dukekunshan.edu.cn/sites/all/modules/jquery_update/replace/jquery/1.7/jquery.min.js?v=1.7.2";
    let response4 = client.get(url4).send().await?;
    let header4 = response4.headers();
    println!("4:{:?}", header4);

    let url5 = "https://dukekunshan.edu.cn/en/printpdf/5638";
    let response5 = client.get(url5).send().await?;
    let header5 = response5.headers();
    println!("5:{:?}", header5);

    let url6 = "https://archlinux.org/static/rss.ca03bf98a65a.svg";
    let response6 = client.get(url6).send().await?;
    let header6 = response6.headers();
    println!("6:{:?}", header6);

    println!(
        "\ncontent type: 0:{}\n1:{}\n3:{}\n4:{}\n5:{}\n6:{}",
        header0.get("content-type").unwrap().to_str()?,
        header1.get("content-type").unwrap().to_str()?,
        // header2.get("content-type").unwrap().to_str()?,
        header3.get("content-type").unwrap().to_str()?,
        header4.get("content-type").unwrap().to_str()?,
        header5.get("content-type").unwrap().to_str()?,
        header6.get("content-type").unwrap().to_str()?
    );

    Ok(())
}
