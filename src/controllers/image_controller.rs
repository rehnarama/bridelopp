use image::io::Reader as ImageReader;
use md5;
use reqwest::{header::HeaderMap, Body};
use rocket::data::{Data, ToByteUnit};
use rocket::http::private::cookie::Expiration;
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
use time::{Duration, OffsetDateTime};
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

const SESSION_COOKIE: &'static str = "session";

pub struct ImageController;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ImageContext {
    images: Vec<Image>,
    session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Image {
    pub url: String,
    pub created: String,
    pub width: u32,
    pub height: u32,
    pub portrait: bool,
    pub owned: bool,
}

impl Image {
    fn from_dao(value: &images::Image, session_id: &str) -> Self {
        Image {
            url: value.url.clone(),
            created: OffsetDateTime::from(value.created.to_system_time())
                .format(format_description!("[year]-[month]-[day] [hour]:[minute]"))
                .unwrap(),
            width: value.width,
            height: value.height,
            portrait: value.width < value.height,
            owned: value.session_id == session_id,
        }
    }
}

#[delete("/<url>")]
async fn delete(
    url: String,
    db: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<(), Error> {
    let session_id = get_session_id(cookies);

    db.remove_image(url, &session_id).await?;

    Ok(())
}

#[put("/", data = "<file>", format = "image/*")]
async fn upload(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
    file: Data<'_>,
    content_type: &ContentType,
    cookies: &CookieJar<'_>,
) -> Result<(), Error> {
    let session_id = get_session_id(cookies);
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

    db.add_image(url, img.width(), img.height(), &session_id)
        .await?;

    Ok(())
}

#[get("/")]
async fn render(
    db: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<Template, Error> {
    let session_id = get_session_id(cookies);

    let images = db
        .get_images()
        .await?
        .iter()
        .map(|value| Image::from_dao(value, &session_id))
        .collect();

    Ok(Template::render(
        "image",
        ImageContext { images, session_id },
    ))
}

fn get_session_id(cookies: &CookieJar<'_>) -> String {
    match cookies.get(&SESSION_COOKIE) {
        Some(cookie) => cookie.value().to_owned(),

        None => {
            let cookie = Cookie::build(SESSION_COOKIE, Uuid::new_v4().to_string())
                .same_site(rocket::http::SameSite::Lax)
                .expires(Expiration::DateTime(
                    OffsetDateTime::now_utc() + Duration::days(30),
                ))
                .finish();

            let id = cookie.value().to_owned();
            cookies.add(cookie);
            id
        }
    }
}

impl Controller for ImageController {
    fn get_routes(&self) -> Vec<Route> {
        routes![render, upload, delete]
    }

    fn get_basepath(&self) -> &str {
        "/photo"
    }
}
