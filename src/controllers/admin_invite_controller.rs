use rocket::http::{CookieJar, Status};
use rocket::Route;
use rocket::response::Redirect;
use rocket_db_pools::Connection;

use crate::db::jostrid_database::invites::{add_invite, Invite};
use crate::db::jostrid_database::JostridDatabase;
use crate::lib::{Controller, azure_oauth};
use rocket::form::Form;

pub struct InviteController;

#[derive(Debug, FromForm)]
struct CreateInviteRequest<'r> {
    name: &'r str,
    password: &'r str,
}

#[post("/", rank = 2, data = "<invite>")]
async fn create_invite(
    invite: Form<CreateInviteRequest<'_>>,
    client: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let bearer_cookie = cookies.get("bearer");

    match bearer_cookie {
        Some(bearer) => {
            azure_oauth::get_user(bearer.value().to_owned()).await?;

            add_invite(
                &client,
                Invite {
                    password: invite.password.to_string(),
                    responses: vec![],
                    plus_one: false
                },
            ).await?;

            Ok(Redirect::to("/admin"))
        }
        None => {
            Err(Status::Unauthorized)
        }
    }
}

impl Controller for InviteController {
    fn get_routes(&self) -> Vec<Route> {
        routes![create_invite]
    }

    fn get_basepath(&self) -> &str {
        "/admin/invite"
    }
}
