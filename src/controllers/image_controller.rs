use image::io::Reader as ImageReader;
use md5;
use reqwest::{header::HeaderMap, Body};
use rocket::data::{Data, ToByteUnit};
use rocket::http::ContentType;
use rocket::Request;
use rocket::{
    http::{Cookie, CookieJar, Status},
    response::Redirect,
    serde::{Deserialize, Serialize},
    Route, State,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;
use std::io::Cursor;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;

extern crate base64;
use crate::db::jostrid_database::images::{self, ImagesDb};
use crate::{
    config::AppConfig, db::jostrid_database::spotify::SpotifyUser, error, lib::Controller,
};
use crate::{
    db::jostrid_database::{spotify::SpotifyDb, JostridDatabase},
    error::Error,
    pkce,
};

pub struct ImageController;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ImageContext {
    images: Vec<Image>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Image {
    pub url: String,
    pub created: String,
    pub width: u32,
    pub height: u32,
    pub portrait: bool,
}

impl From<images::Image> for Image {
    fn from(value: images::Image) -> Self {
        Image {
            url: value.url,
            created: OffsetDateTime::from(value.created.to_system_time())
                .format(format_description!("[year]-[month]-[day] [hour]:[minute]"))
                .unwrap(),
            width: value.width,
            height: value.height,
            portrait: value.width < value.height,
        }
    }
}

#[delete("/<url>")]
async fn delete(
    url: String,
    db: Connection<JostridDatabase>,
) -> Result<(), Error> {
    db.remove_image(url).await?;
    
    Ok(())
}

#[put("/", data = "<file>", format = "image/*")]
async fn upload(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
    file: Data<'_>,
    content_type: &ContentType,
) -> Result<(), Error> {
    let client = reqwest::Client::new();

    let bytes = file
        .open(100.megabytes())
        .into_bytes()
        .await
        .unwrap()
        .into_inner();
    let img = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()?
        .decode()?;
    let filename = format!(
        "{:x}.{}",
        md5::compute(&bytes),
        content_type.0.extension().unwrap()
    );

    let url = format!("{}{}", config.blob.url, filename);

    let _response = client
        .put(format!("{}{}", url, config.blob.query))
        .body(bytes)
        .header("x-ms-blob-type", "BlockBlob")
        .header("Content-Type", content_type.0.to_string())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    db.add_image(url, img.width(), img.height()).await?;

    Ok(())
}

#[get("/")]
async fn render(db: Connection<JostridDatabase>) -> Result<Template, Error> {
    let images = db
        .get_images()
        .await?
        .iter()
        .map(|i| i.clone().into())
        .collect();

    Ok(Template::render("image", ImageContext { images }))
}

impl Controller for ImageController {
    fn get_routes(&self) -> Vec<Route> {
        routes![render, upload, delete]
    }

    fn get_basepath(&self) -> &str {
        "/photo"
    }
}
