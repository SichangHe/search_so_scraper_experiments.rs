use image_scraper::save_all_image;

fn main() {
    let url0 = "https://dukekunshan.edu.cn";
    let record = save_all_image(url0).unwrap_or_else(|err| {
        eprintln!("{}", err);
        String::from("nothing")
    });
    println!("{}", record);
}
