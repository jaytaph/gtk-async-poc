use std::time::Duration;
use gtk4::gdk_pixbuf::{Colorspace, Pixbuf};
use log::info;
use reqwest::{Client, Error, Response};

const GOSUB_USERAGENT_STRING: &str = "Mozilla/5.0 (X11; Linux x86_64; Wayland; rv:1.0) Gecko/20231106 Gosub/0.1 Firefox/89.0";

pub async fn fetch_url_body(url: &str) -> Result<Vec<u8>, Error> {
    match fetch_url(url).await {
        Ok(response) => {
            let body = response.bytes().await?.to_vec();
            Ok(body)
        }
        Err(e) => Err(e),
    }
}

pub async fn fetch_url(url: &str) -> Result<Response, Error> {
    let client = Client::builder()
        .user_agent(GOSUB_USERAGENT_STRING)
        .timeout(Duration::from_secs(5))
        .build()?;

    client.get(url).send().await
}

pub async fn fetch_favicon(url: &str) -> Vec<u8> {
    let url = format!("{}{}", url, "/favicon.ico");
    let Ok(buf) = fetch_url_body(url.as_str()).await else {
        info!("Failed to fetch favicon from URL");
        return Vec::new();
    };

    buf
}

pub fn bytes_to_pixbuf(buf: Vec<u8>) -> Option<Pixbuf> {
    let Ok(img) = image::load_from_memory(&buf) else {
        info!("Failed to load favicon into buffer (image)");
        return None;
    };

    // Convert to RGBA format if not already
    let rgba_image = img.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    let pixels = rgba_image.into_raw();

    // Create a Pixbuf from the raw RGBA data
    let pixbuf = Pixbuf::from_mut_slice(
        pixels,
        Colorspace::Rgb,
        true, // Has alpha channel
        8,    // Bits per channel
        width as i32,
        height as i32,
        width as i32 * 4,
    );

    Some(pixbuf)
}