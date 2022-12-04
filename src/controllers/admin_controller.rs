use rocket::http::{Cookie, CookieJar, Status};
use rocket::State;
use rocket::{serde::Serialize, Route};
use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;

use crate::config::AppConfig;
use crate::db::jostrid_database::invites::{self, Response};
use crate::db::jostrid_database::JostridDatabase;
use crate::lib::azure_oauth::{self, MeResponse};
use crate::lib::Controller;
use crate::pkce;

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
    config: &State<AppConfig>,
    cookies: &CookieJar<'_>,
) -> Result<Template, Status> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => match azure_oauth::get_user(bearer.value().to_owned()).await {
            Ok(user) => render_admin(&client, &user, bearer.value().to_string()).await,
            Err(_) => render_login(config, cookies),
        },
        None => render_login(config, cookies),
    }
}

async fn render_admin(
    client: &Connection<JostridDatabase>,
    user: &MeResponse,
    bearer: String,
) -> Result<Template, Status> {
    let invites = invites::get_invites(client)
        .await?
        .iter()
        .map(|i| i.into())
        .collect();

    Ok(Template::render(
        "admin",
        AdminContext {
            bearer: bearer,
            name: user.display_name.clone(),
            email: user.user_principal_name.clone(),
            invites,
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

impl Controller for AdminController {
    fn get_routes(&self) -> Vec<Route> {
        routes![admin]
    }

    fn get_basepath(&self) -> &str {
        "/admin"
    }
}
