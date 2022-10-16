use rocket::{
    http::{Cookie, CookieJar, Status},
    response::Redirect,
    serde::{Deserialize, Serialize},
    Route, State,
};

use crate::{config::AppConfig, lib::Controller};

pub struct AzureOauthController;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OauthTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
    scope: String,
}

#[get("/callback?<code>&<state>", rank = 2)]
async fn oauth_callback(
    cookies: &CookieJar<'_>,
    code: &str,
    state: &str,
    config: &State<AppConfig>,
) -> Result<Redirect, Status> {
    let default_cookie = Cookie::new("default", "");

    let verifier = cookies.get("code_verifier").unwrap_or(&default_cookie);
    let stored_state = cookies.get("state").unwrap_or(&default_cookie);
    assert_eq!(state, stored_state.value());

    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".to_string();

    let params = [
        ("client_id", config.azure.client_id.to_owned()),
        ("scope", "User.Read".to_owned()),
        ("code", code.to_owned()),
        ("redirect_uri", config.azure.redirect_uri.to_owned()),
        ("grant_type", "authorization_code".to_owned()),
        ("code_verifier", verifier.value().to_owned()),
        ("client_secret", config.azure.secret.to_owned()),
    ];
    let client = reqwest::Client::new();

    let resp = match client.post(url).form(&params).send().await {
        Ok(response) => match response.json::<OauthTokenResponse>().await {
            Ok(body) => Ok(body),
            Err(e) => {
                error!("Error: {}", e);
                Err(Status::InternalServerError)
            }
        },
        Err(e) => {
            error!("Error: {}", e);
            Err(Status::InternalServerError)
        }
    }?;

    cookies.add(
        Cookie::build("bearer", resp.access_token)
            .same_site(rocket::http::SameSite::Lax)
            .finish(),
    );

    Ok(Redirect::to("/admin"))
}

impl Controller for AzureOauthController {
    fn get_routes(&self) -> Vec<Route> {
        routes![oauth_callback]
    }

    fn get_basepath(&self) -> &str {
        "/oauth/azure"
    }
}
