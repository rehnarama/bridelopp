use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::Route;
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;

use crate::db::jostrid_database::invites::{self, add_invite, add_responses, Invite};
use crate::db::jostrid_database::JostridDatabase;
use crate::lib::authentication::get_invite;
use crate::lib::{azure_oauth, Controller};
use rocket::form::Form;

use super::template_controller::MainContext;

pub struct RegistrationController;

#[derive(Debug, FromForm)]
struct Response {
    pub name: String,
    pub attending: bool,
    pub food_preferences: String
}

impl From<&Response> for invites::Response {
    fn from(r: &Response) -> invites::Response {
        invites::Response {
            name: r.name.clone(),
            attending: r.attending,
            food_preferences: Some(r.food_preferences.clone())
        }
    }
}

#[derive(Debug, FromForm)]
struct CreateResponsesRequest {
    responses: Vec<Response>,
    plus_one: bool
}

#[post("/", rank = 2, data = "<body>")]
async fn create_response(
    body: Form<CreateResponsesRequest>,
    client: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    dbg!(&body);

    let password_cookie = cookies.get("password");

    let invite = get_invite(&client, password_cookie).await?;

    let responses: Vec<invites::Response> = body
        .responses
        .iter()
        .map(|r| invites::Response::from(r))
        .collect();

    add_responses(&client, &invite.password, responses, body.plus_one).await?;

    Ok(Redirect::to("/registration"))
}

impl Controller for RegistrationController {
    fn get_routes(&self) -> Vec<Route> {
        routes![create_response]
    }

    fn get_basepath(&self) -> &str {
        "/registration"
    }
}
