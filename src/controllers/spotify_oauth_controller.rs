use reqwest::header::HeaderMap;
use rocket::{
    http::{Cookie, CookieJar, Status},
    response::Redirect,
    serde::{Deserialize, Serialize},
    Route, State,
};
use rocket_db_pools::Connection;

extern crate base64;
use crate::{
    config::AppConfig, db::jostrid_database::spotify::SpotifyUser, error, lib::Controller,
};
use crate::{
    db::jostrid_database::{spotify::SpotifyDb, JostridDatabase},
    error::Error,
    pkce,
};

pub struct SpotifyOauthController;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OauthTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
    scope: String,
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct RefreshTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
    scope: String,
}

#[get("/callback?<code>&<state>", rank = 2)]
async fn oauth_callback(
    db: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
    code: &str,
    state: &str,
    config: &State<AppConfig>,
) -> Result<Redirect, Error> {
    let default_cookie = Cookie::new("default", "");

    let stored_state = cookies.get("state").unwrap_or(&default_cookie);
    assert_eq!(state, stored_state.value());

    let url = "https://accounts.spotify.com/api/token".to_owned();

    let params = [
        ("code", code.to_owned()),
        ("redirect_uri", config.spotify.redirect_uri.to_owned()),
        ("grant_type", "authorization_code".to_owned()),
    ];
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!(
            "Basic {}",
            base64::encode(format!(
                "{}:{}",
                config.spotify.client_id, config.spotify.secret
            ))
        )
        .parse()
        .unwrap(),
    );

    let client = reqwest::Client::new();

    let resp = match client.post(url).form(&params).headers(headers).send().await {
        Ok(response) => Ok(response.json::<OauthTokenResponse>().await),
        Err(e) => Err(e),
    }??;

    db.insert_user(
        resp.access_token,
        resp.refresh_token,
        resp.expires_in as i64,
    )
    .await?;

    Ok(Redirect::to(uri!("/admin")))
}

pub async fn refresh_token(
    db: Connection<JostridDatabase>,
    config: &State<AppConfig>,
) -> Result<SpotifyUser, error::Error> {
    let url = "https://accounts.spotify.com/api/token".to_owned();

    let user = match db.get_user().await.unwrap() {
        None => {
            info!("No spotify user registered");
            return Err(error::Error {
                status: Status::NotFound,
                msg: Some("No spotify user found".to_owned()),
            });
        }
        Some(user) => user,
    };

    let params = [
        ("refresh_token", user.refresh_token.to_owned()),
        ("grant_type", "refresh_token".to_owned()),
    ];
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!(
            "Basic {}",
            base64::encode(format!(
                "{}:{}",
                config.spotify.client_id, config.spotify.secret
            ))
        )
        .parse()
        .unwrap(),
    );

    let client = reqwest::Client::new();

    let resp = match client.post(url).form(&params).headers(headers).send().await {
        Ok(response) => Ok(response.json::<RefreshTokenResponse>().await.map_err(|e| {
            error!("Error: {}", e);
            Status::InternalServerError
        })),
        Err(e) => {
            error!("Error: {}", e);
            Err(Status::InternalServerError)
        }
    }??;

    Ok(db
        .refresh_user(
            resp.access_token,
            user.refresh_token,
            resp.expires_in as i64,
        )
        .await?)
}

#[get("/authorize?<response_type>&<client_id>&<scope>&<redirect_uri>&<state>")]
fn authorize(response_type: &str, client_id: &str, scope: &str, redirect_uri: &str, state: &str) {}

#[get("/login", rank = 2)]
async fn oauth_login(
    cookies: &CookieJar<'_>,
    config: &State<AppConfig>,
) -> Result<Redirect, Status> {
    let state = pkce::random_state(32);
    cookies.add(
        Cookie::build("state", state.to_string())
            .same_site(rocket::http::SameSite::Lax)
            .finish(),
    );

    let scope = "app-remote-control playlist-read-private playlist-read-collaborative playlist-modify-public playlist-modify-private user-library-read user-library-modify user-read-private user-read-email user-follow-read user-follow-modify user-top-read user-read-playback-position user-read-playback-state user-read-recently-played user-read-currently-playing user-modify-playback-state ugc-image-upload streaming";

    Ok(Redirect::to(uri!(
        "https://accounts.spotify.com",
        authorize(
            "code",
            config.spotify.client_id.to_owned(),
            scope,
            config.spotify.redirect_uri.to_owned(),
            state
        )
    )))
}

impl Controller for SpotifyOauthController {
    fn get_routes(&self) -> Vec<Route> {
        routes![oauth_callback, oauth_login]
    }

    fn get_basepath(&self) -> &str {
        "/oauth/spotify"
    }
}
