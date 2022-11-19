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

pub struct ResponseController;

#[derive(Debug, FromForm)]
struct Response {
    pub name: String,
    pub attending: bool,
}

impl From<&Response> for invites::Response {
    fn from(r: &Response) -> invites::Response {
        invites::Response {
            name: r.name.clone(),
            attending: r.attending,
        }
    }
}

#[derive(Debug, FromForm)]
struct CreateResponsesRequest {
    responses: Vec<Response>,
}

#[post("/", rank = 2, data = "<body>")]
async fn create_response(
    body: Form<CreateResponsesRequest>,
    client: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let password_cookie = cookies.get("password");

    let invite = get_invite(&client, password_cookie).await?;

    let responses: Vec<invites::Response> = body
        .responses
        .iter()
        .map(|r| invites::Response::from(r))
        .collect();

    add_responses(&client, &invite.password, responses).await?;

    Ok(Redirect::to("/registration"))
}

impl Controller for ResponseController {
    fn get_routes(&self) -> Vec<Route> {
        routes![create_response]
    }

    fn get_basepath(&self) -> &str {
        "/response"
    }
}
