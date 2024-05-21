use original_image_scraper::save_all_image;
use std::io::{self, BufRead};

/// # Scrape and save original image
/// This program take *line of URL* from *stdin*
/// and download all the image referenced in them
/// preserve the file extension and name the file as its sha256
/// **return** a string of {URL}:{sha256.extension} pair
fn main() {
    let stdin =io::stdin();
    let input = stdin.lock().lines();
    let mut record = String::new();
    for line in input {
        match line {
            Err(err) => eprintln!("{}", err),
            Ok(url) => {
                record.push_str(&save_all_image(&url).unwrap_or_else(|err| {
                    eprintln!("{}", err);
                    String::from("nothing")
                }));
            }
        }
    }
    println!("{}", record);
}
