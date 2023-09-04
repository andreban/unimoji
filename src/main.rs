use env_logger::Env;
use image::{io::Reader as ImageReader, GenericImageView};
use ledstrip::LedStrip;
use reqwest::ClientBuilder;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::Duration;

mod ledstrip;

#[derive(Debug, Deserialize)]
struct Config {
    pub spi_dev: String,
    pub emoji_directory: String,
    pub firebase_url: String,
}

#[derive(Debug, Deserialize)]
struct PayloadData {
    emoji: String,
}

#[derive(Debug, Deserialize)]
struct Payload {
    data: PayloadData,
}

fn parse_chunk_line(input: &str) -> io::Result<(&str, &str)> {
    let parts = input.splitn(2, ':').map(|s| s.trim()).collect::<Vec<_>>();

    if parts.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid input"));
    }

    Ok(((parts[0]), (parts[1])))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("unimoji=debug")).init();
    let config = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config)?;

    let mut led_strip = LedStrip::open(&config.spi_dev)?;
    let client = ClientBuilder::new().build()?;

    loop {
        let response = client
            .get(&config.firebase_url)
            .header("Accept", "text/event-stream")
            .send()
            .await;

        let mut response = match response {
            Ok(response) => response,
            Err(e) => {
                log::error!("Error requesting from server: {}", e);
                thread::sleep(Duration::from_millis(1000));
                continue;
            }
        };

        loop {
            let Ok(chunk) = tokio::time::timeout(Duration::from_secs(60), response.chunk()).await
            else {
                log::error!("Timed out getting chunk");
                break;
            };

            let Ok(Some(chunk)) = chunk else {
                log::error!("Failed to get chunk");
                break;
            };

            let chunk_vec = chunk.to_vec();
            let chunk_str = String::from_utf8_lossy(&chunk_vec);
            let lines = chunk_str.lines().collect::<Vec<_>>();
            if lines.len() < 2 {
                log::error!("Not enough lines. Skipping...");
            }

            let (_, command) = parse_chunk_line(lines[0])?;
            if command == "put" {
                let (_, data) = parse_chunk_line(lines[1])?;
                let emoji = serde_json::from_str::<Payload>(data).unwrap().data.emoji;
                log::info!("Received emoji: {}", emoji);
                let unicode = emoji
                    .escape_unicode()
                    .to_string()
                    .replacen("\\u", "emoji_u", 1)
                    .replace("\\u", "_")
                    .replace(['{', '}'], "");

                let mut filename = config.emoji_directory.to_string() + "/" + &unicode + ".png";
                if !Path::new(&filename).exists() {
                    let previous_unicode = unicode.rsplitn(2, '_').last().unwrap();
                    filename = config.emoji_directory.to_string() + "/" + previous_unicode + ".png";
                }

                let img = match load_image(filename) {
                    Ok(img) => img,
                    Err(e) => {
                        log::error!("Error loading image: {}", e);
                        continue;
                    }
                };

                if let Err(e) = led_strip.send_image(&img) {
                    log::error!("Failed to send image to Unicorn hat: {}", e);
                }
            }
        }
    }
}

fn load_image<P: AsRef<Path>>(filename: P) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(ImageReader::open(filename)?
        .decode()?
        .resize(16, 16, image::imageops::FilterType::Nearest)
        .pixels()
        .flat_map(|(_, _, rgba)| vec![rgba[0], rgba[1], rgba[2]])
        .collect::<Vec<_>>())
}
