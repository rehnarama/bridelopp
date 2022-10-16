use rocket::http::{Cookie, CookieJar, Status};
use rocket::State;
use rocket::{serde::Serialize, Route};
use rocket_db_pools::mongodb::bson::doc;
use rocket_dyn_templates::Template;

use crate::config::AppConfig;
use crate::lib::azure_oauth;
use crate::lib::Controller;
use crate::pkce;

pub struct AdminController;

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
async fn admin<'r>(config: &State<AppConfig>, cookies: &CookieJar<'_>) -> Result<Template, Status> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => {
            let user = azure_oauth::get_user(bearer.value().to_owned()).await?;

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

impl Controller for AdminController {
    fn get_routes(&self) -> Vec<Route> {
        routes![admin]
    }

    fn get_basepath(&self) -> &str {
        "/admin"
    }
}
