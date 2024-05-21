use bytes::Bytes;
use reqwest::Client;
use select::{document::Document, predicate::Name};
use sha256::digest;
use std::{error::Error, fs::File, io::Write, time::Duration};
use tokio::spawn;
use url::Url;

#[tokio::main]
pub async fn crawl_links_r(url0: Url) -> Result<Option<Vec<Url>>, Box<dyn Error>> {
    let mut checked_urls = Vec::new();
    let mut links_waitlist = Vec::new();

    links_waitlist.push(vec![url0]);

    loop {
        // pop one list of links and process all of them
        if let Some(more_links) = links_waitlist.pop() {
            let mut handles = Vec::new();
            // process the links
            for link in more_links {
                match checked_urls.binary_search(&link) {
                    // start a new async process for new link if not checked
                    Err(_) => handles.push(spawn(async move {
                        match crawl_links(&link).await {
                            Ok(result) => Some(result),
                            Err(err) => {
                                eprintln!("Master: {}", err);
                                None
                            }
                        }
                    })),

                    // link already checked
                    Ok(_) => (),
                }
            }
            for handle in handles {
                match handle.await.unwrap_or_else(|err| {
                    eprintln!("Master: {}", err);
                    None
                }) {
                    // get outcome of each process
                    Some((final_url, path, links_op)) => {
                        println!("\"{}\": \"{}\"", final_url, path); // output "url": "path"
                        checked_urls.push(final_url);
                        if let Some(links) = links_op {
                            links_waitlist.push(links)
                        }
                    }
                    None => (),
                }
            }

            // clean up after one loop
            checked_urls.sort();
            checked_urls.dedup();

            eprintln!(
                "Master: Summary after one round: {} URLs checked\n",
                checked_urls.len()
            );
        } else {
            break;
        }
    }

    Ok(Some(checked_urls))
}

/// HTTP request given URL, save it as HTML file if it is,\
/// or use its file extension in its URL\
/// **File name** is the sha256 of the URL\
/// # Return
/// `(final_url, path, links)`\
/// `final_url`: the URL after potential redirection\
/// `path`: the saved file's name with extension\
/// `links`: links (hrefs + imgs) from the URL if any
pub async fn crawl_links(url: &Url) -> Result<(Url, String, Option<Vec<Url>>), Box<dyn Error>> {
    eprintln!("Requesting {}", url);
    let path: String;

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5)) // timeout: 5 sec
        .build()?;

    let response = client.get(url.clone()).send().await?;
    let final_url = response.url().to_owned(); // URL after potential redirection
    let hash = digest(final_url.as_str()); // for file name

    // check file type: HTML or other
    let header = response
        .headers()
        .get("content-type")
        .unwrap_or_else(|| panic!("No content-type of {} found!", url))
        .to_str()?;
    if header.contains("text/html") {
        // type: HTML
        let mut links = Vec::new();
        let html = response.text().await?;

        // iterate through all the href and store them in `hrefs`
        for href in Document::from(html.as_str())
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
        {
            let href_url = final_url.join(href);
            match href_url {
                Ok(href_url0) => links.push(href_url0),
                Err(err) => eprintln!("{} | {}", err, url),
            }
        }

        // iterate through all the img src and store them in imgs
        for img in Document::from(html.as_str())
            .find(Name("img"))
            .filter_map(|n| n.attr("src"))
        {
            let img_url = final_url.join(img);
            match img_url {
                Ok(img_url0) => links.push(img_url0),
                Err(err) => eprintln!("{} | {}", err, url),
            }
        }
        links.sort();
        links.dedup();

        // save the HTML file
        path = hash + ".html";
        save_html(&path, html).unwrap_or_else(|err| eprintln!("{} | {}", err, url));

        Ok((final_url, path, Some(links)))

        //
    } else {
        // type: other
        eprintln!("Content-type: {} | {}", header, url);

        // set file extension according to URL
        let file_extension = match url.as_str().split(".").last() {
            None => {
                eprintln!("file format unknown | {}", url);
                String::new()
            }
            Some(extension) => format!(".{}", extension),
        };

        // save the file with given extension
        path = hash + &file_extension;
        save_file(&path, response.bytes().await?)
            .unwrap_or_else(|err| eprintln!("{} | {}", err, url));

        Ok((final_url, path, None))
    }
}

fn save_file(path: &str, file_bytes: Bytes) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(&path)?;
    file.write_all(&file_bytes)?;
    Ok(())
}

fn save_html(path: &str, html: String) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(&path)?;
    write!(file, "{}", html)?;
    Ok(())
}
