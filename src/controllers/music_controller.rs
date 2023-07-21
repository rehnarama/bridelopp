use std::path::PathBuf;

use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rocket::State;
use rocket::{serde::Serialize, Route};
use rocket_db_pools::mongodb::bson::{doc, DateTime};
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;

use crate::config::AppConfig;
use crate::db::jostrid_database::invites::Invite;
use crate::db::jostrid_database::spotify::{QueueItem, SpotifyDb};
use crate::db::jostrid_database::{invites, JostridDatabase};
use crate::error::Error;
use crate::lib::{authentication, Controller};

use super::spotify_oauth_controller::refresh_token;

#[derive(Debug, FromForm)]
struct LoginRequest<'r> {
    password: &'r str,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicContext {}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct ArtistDto {
    href: String,
    id: String,
    images: Option<Vec<ImageObjectDto>>,
    name: String,
    uri: String,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct ImageObjectDto {
    url: String,
    height: Option<u32>,
    width: Option<u32>,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct AlbumDto {
    href: String,
    id: String,
    images: Option<Vec<ImageObjectDto>>,
    name: String,
}
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct TrackObjectDto {
    album: AlbumDto,
    artists: Vec<ArtistDto>,
    duration_ms: u32,
    href: String,
    id: String,
    name: String,
    uri: String,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct TrackDto {
    href: String,
    limit: u32,
    next: Option<String>,
    offset: u32,
    previous: Option<String>,
    total: u32,
    items: Vec<TrackObjectDto>,
}
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct SearchDto {
    tracks: TrackDto,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VoteDto {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub image: String,
}

pub struct MusicController;

async fn get_token(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
) -> Result<String, Error> {
    match db.get_user().await? {
        Some(user) => {
            if user.expires < DateTime::now() {
                Ok(refresh_token(db, config).await?.access_token)
            } else {
                Ok(user.access_token)
            }
        }
        None => Err(Error {
            status: Status::NotFound,
            msg: Some("No spotify user authenticated".to_owned()),
        }),
    }
}

#[get("/api/search?<query>")]
async fn search(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
    query: String,
) -> Result<Json<SearchDto>, Error> {
    let token = get_token(db, config).await?;

    let client = reqwest::Client::new();
    let url = "https://api.spotify.com/v1/search";

    let response = client
        .get(url)
        .query(&[("q", query), ("type", "track".to_owned())])
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    Ok(Json(response.json::<SearchDto>().await?))
}

#[get("/vote/<uri>")]
async fn get_vote(db: Connection<JostridDatabase>, uri: String) -> Result<Json<QueueItem>, Error> {
    match db.get_queue_item(uri).await? {
        Some(queue_item) => Ok(Json(queue_item)),
        None => Err(Status::NotFound.into()),
    }
}

#[get("/vote")]
async fn get_votes(db: Connection<JostridDatabase>) -> Result<Json<Vec<QueueItem>>, Error> {
    match db.get_user().await? {
        Some(user) => Ok(Json(user.queue)),
        None => Err(Status::NotFound.into()),
    }
}

#[post("/vote", data = "<vote>")]
async fn vote(db: Connection<JostridDatabase>, vote: Json<VoteDto>) -> Result<(), Error> {
    Ok(db.add_vote(vote.0).await?)
}

#[get("/music")]
async fn render<'r>(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
) -> Result<Template, Status> {
    Ok(Template::render("music", MusicContext {}))
}

impl Controller for MusicController {
    fn get_routes(&self) -> Vec<Route> {
        routes![render, search, get_vote, get_votes, vote]
    }

    fn get_basepath(&self) -> &str {
        "/"
    }
}
