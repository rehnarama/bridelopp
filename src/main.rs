mod pkce;

#[macro_use]
extern crate rocket;
use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::{FileServer, NamedFile};
use rocket::http::{impl_from_uri_param_identity, Cookie, CookieJar, Status};
use rocket::log::private::error;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_dyn_templates::Template;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use rocket_db_pools::mongodb::{bson::doc, Client};
use rocket_db_pools::{Connection, Database};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct AppConfig {
    azure: AzureConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct AzureConfig {
    secret: String,
    client_id: String,
    redirect_uri: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LoginContext;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct MainContext {
    invite: Invite,
    route: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct AzureContext {
    client_id: String,
    redirect_uri: String,
    scopes: String,
    state: String,
    challenge: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct AdminContext {
    bearer: String,
    name: String,
    email: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct AdminLoginContext {
    azure: AzureContext,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Invite {
    password: String,
    name: String,
}

#[derive(Debug, FromForm)]
struct LoginRequest<'r> {
    password: &'r str,
}

#[get("/public/<file..>", rank = 1)]
async fn public(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("public/").join(file)).await.ok()
}

#[get("/admin", rank = 2)]
async fn admin<'r>(config: &State<AppConfig>, cookies: &CookieJar<'_>) -> Result<Template, Status> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => {
            let user = get_user(bearer.value().to_owned()).await?;

            Ok(Template::render(
                "admin",
                AdminContext {
                    bearer: bearer.value().to_owned(),
                    name: user.display_name,
                    email: user.user_principal_name,
                },
            ))
        }
        None => {
            let state = pkce::random_state(32);
            // Generate a random 128-byte code verifier (must be between 43 and 128 bytes)
            let code_verify = pkce::code_verifier(128);
            // Generate an encrypted code challenge accordingly
            let code_challenge = pkce::code_challenge(&code_verify);

            cookies.add(
                Cookie::build("state", state.to_string())
                    .same_site(rocket::http::SameSite::Lax)
                    .finish(),
            );
            cookies.add(
                Cookie::build("code_verifier", pkce::code_verifier_string(code_verify))
                    .same_site(rocket::http::SameSite::Lax)
                    .finish(),
            );

            Ok(Template::render(
                "admin_login",
                AdminLoginContext {
                    azure: AzureContext {
                        client_id: urlencoding::encode(&config.azure.client_id).to_string(),
                        redirect_uri: urlencoding::encode(&config.azure.redirect_uri).to_string(),
                        scopes: urlencoding::encode("User.Read").to_string(),
                        state: urlencoding::encode(&state).to_string(),
                        challenge: urlencoding::encode(&code_challenge).to_string(),
                    },
                },
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OauthTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
    scope: String,
}

#[get("/oauth/azure/callback?<code>&<state>", rank = 2)]
async fn oauth_callback(
    cookies: &CookieJar<'_>,
    code: &str,
    state: &str,
    config: &State<AppConfig>,
) -> Result<Redirect, Status> {
    let default_cookie = Cookie::new("default", "");

    let verifier = cookies.get("code_verifier").unwrap_or(&default_cookie);
    let stored_state = cookies.get("state").unwrap_or(&default_cookie);
    dbg!(verifier);
    dbg!(stored_state);
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "camelCase")]
struct MeResponse {
    display_name: String,
    user_principal_name: String,
}

async fn get_user(bearer: String) -> Result<MeResponse, Status> {
    let url = "https://graph.microsoft.com/v1.0/me";

    let client = reqwest::Client::new();
    let me_response = match client.get(url).bearer_auth(bearer).send().await {
        Ok(response) => match response.json::<MeResponse>().await {
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

    let allowed_emails = vec!["astrid.rehn@outlook.com", "josefin.ahlenius@hotmail.com"];

    let email: &str = &me_response.user_principal_name;
    if allowed_emails.contains(&email) {
        Ok(me_response)
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/<path..>", rank = 3)]
async fn index<'r>(
    path: PathBuf,
    client: Connection<Db>,
    cookies: &CookieJar<'_>,
) -> Result<Template, Status> {
    let path_str = path.to_str().unwrap_or("main").to_string();

    let resolved = if path_str.is_empty() {
        "main".to_string()
    } else if path_str.eq("registration") {
        path_str
    } else if path_str.eq("information") {
        path_str
    } else if path_str.eq("contact") {
        path_str
    } else {
        return Err(Status::NotFound);
    };

    match cookies.get("password") {
        Some(password) => match get_invite(client, password.value().to_string()).await? {
            Some(invite) => Ok(Template::render(
                resolved.clone(),
                MainContext {
                    invite,
                    route: resolved.clone(),
                },
            )),
            None => Err(Status::InternalServerError),
        },
        None => Ok(Template::render("login", LoginContext)),
    }
}

#[derive(Database)]
#[database("jostrid")]
pub struct Db(Client);

#[post("/", data = "<request>")]
async fn login<'r>(
    request: Form<LoginRequest<'r>>,
    client: Connection<Db>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let password = request.password.to_string();

    match get_invite(client, password.clone()).await? {
        Some(invite) => {
            cookies.add(Cookie::new("password", password));

            Ok(invite)
        }
        None => {
            println!("Didn't find invite with password {}", password.clone());
            Err(Status::NotFound)
        }
    }?;

    Ok(Redirect::to("/"))
}

async fn get_invite(client: Connection<Db>, password: String) -> Result<Option<Invite>, Status> {
    client
        .deref()
        .database("jostrid")
        .collection::<Invite>("invites")
        .find_one(doc! { "password": password }, None)
        .await
        .map_err(|_| Status::InternalServerError)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::init())
        .mount("/", routes![public, index, admin, login, oauth_callback])
        .attach(Template::fairing())
        .attach(AdHoc::config::<AppConfig>())
}
