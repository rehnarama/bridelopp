use std::sync::atomic::Ordering;

use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket::{serde::Serialize, Route};
use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;

use crate::config::AppConfig;
use crate::controllers::music_controller::MusicController;
use crate::db::jostrid_database::invites::{self, Response};
use crate::db::jostrid_database::spotify::{QueueItem, SpotifyDb, SpotifyUser};
use crate::db::jostrid_database::JostridDatabase;
use crate::error::Error;
use crate::lib::azure_oauth::{self, MeResponse};
use crate::lib::Controller;
use crate::pkce;

use super::music_controller;

pub struct AdminController;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Invite {
    password: String,
    responses: Vec<Response>,
    plus_one: bool,
    submitted: bool,
    greeting: String,
    first_login: Option<String>,
    last_login: Option<String>,
    address: String,
}

impl From<&invites::Invite> for Invite {
    fn from(invite: &invites::Invite) -> Self {
        Invite {
            password: invite.password.clone(),
            responses: invite.responses.clone(),
            plus_one: invite.plus_one,
            submitted: invite.submitted,
            greeting: invite.greeting.clone(),
            first_login: invite.first_login.map(|date| {
                date.try_to_rfc3339_string()
                    .unwrap_or("Error converting date to string".to_string())
            }),
            last_login: invite.last_login.map(|date| {
                date.try_to_rfc3339_string()
                    .unwrap_or("Error converting date to string".to_string())
            }),
            address: invite.address.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct AdminContext {
    bearer: String,
    name: String,
    email: String,
    invites: Vec<Invite>,
    n_attending: usize,
    n_not_attending: usize,
    n_no_response: usize,
    n_total: usize,
    queue_active: bool,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct AdminLoginContext {
    azure: AzureContext,
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

#[get("/", rank = 2)]
async fn admin<'r>(
    client: Connection<JostridDatabase>,
    music_controller: &State<MusicController>,
    config: &State<AppConfig>,
    cookies: &CookieJar<'_>,
) -> Result<Template, Status> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => match azure_oauth::get_user(bearer.value().to_owned()).await {
            Ok(user) => {
                render_admin(&client, music_controller, &user, bearer.value().to_string()).await
            }
            Err(_) => render_login(config, cookies),
        },
        None => render_login(config, cookies),
    }
}

#[delete("/vote/<uri>")]
async fn vote(db: Connection<JostridDatabase>, uri: String) -> Result<(), Error> {
    db.delete_queue_item(uri).await?;

    Ok(())
}

async fn render_admin(
    client: &Connection<JostridDatabase>,
    music_controller: &State<MusicController>,
    user: &MeResponse,
    bearer: String,
) -> Result<Template, Status> {
    let invites: Vec<Invite> = invites::get_invites(client)
        .await?
        .iter()
        .map(|i| i.into())
        .collect();

    let all_responses: Vec<&Response> = invites.iter().flat_map(|i| &i.responses).collect();
    let all_submitted_responses: Vec<&Response> = invites
        .iter()
        .filter(|i| i.submitted)
        .flat_map(|i| &i.responses)
        .collect();
    let n_total = all_responses.iter().count();
    let n_attending = all_submitted_responses
        .iter()
        .filter(|r| r.attending)
        .map(|_| 1)
        .sum();
    let n_not_attending = all_submitted_responses
        .iter()
        .filter(|r| !r.attending)
        .map(|_| 1)
        .sum();
    let n_no_response = n_total - (n_attending + n_not_attending);

    Ok(Template::render(
        "admin",
        AdminContext {
            bearer: bearer,
            name: user.display_name.clone(),
            email: user.user_principal_name.clone(),
            invites,
            n_attending,
            n_not_attending,
            n_no_response,
            n_total,
            queue_active: music_controller.queue_active.load(Ordering::Relaxed),
        },
    ))
}

fn render_login(config: &State<AppConfig>, cookies: &CookieJar<'_>) -> Result<Template, Status> {
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

#[derive(FromForm)]
struct QueueData {
    enabled: bool,
}

#[post("/queue", data = "<data>")]
async fn activate_queue(
    data: Form<QueueData>,
    controller: &State<MusicController>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Error> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => Ok::<(), Error>(
            azure_oauth::get_user(bearer.value().to_owned())
                .await
                .and_then(|_| match data.enabled {
                    true => Ok(controller.activate_queue()),
                    false => Ok(controller.pause_queue()),
                })?,
        ),
        None => Err(Status::Unauthorized.into()),
    }?;

    Ok(Redirect::to(uri!("/admin")))
}

impl Controller for AdminController {
    fn get_routes(&self) -> Vec<Route> {
        routes![admin, activate_queue, vote]
    }

    fn get_basepath(&self) -> &str {
        "/admin"
    }
}
