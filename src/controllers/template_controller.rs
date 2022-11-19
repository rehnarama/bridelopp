use std::path::PathBuf;

use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::{serde::Serialize, Route};
use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::Connection;
use rocket_dyn_templates::Template;

use crate::db::jostrid_database::invites::{Invite};
use crate::db::jostrid_database::{invites, JostridDatabase};
use crate::lib::{Controller, authentication};

#[derive(Debug, FromForm)]
struct LoginRequest<'r> {
    password: &'r str,
}

#[post("/login", data = "<request>")]
async fn login<'r>(
    request: Form<LoginRequest<'r>>,
    client: Connection<JostridDatabase>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let password = request.password.to_string();

    match invites::get_invite(&client, password.clone()).await? {
        Some(invite) => {
            cookies.add(Cookie::new("password", password));

            Ok(invite)
        }
        None => {
            error!("Didn't find invite with password {}", password.clone());
            Err(Status::NotFound)
        }
    }?;

    Ok(Redirect::to("/"))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginContext;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct MainContext {
    invite: Invite,
    route: String,
}

pub struct TemplateController;

#[get("/<path..>", rank = 3)]
async fn get_template<'r>(
    path: PathBuf,
    client: Connection<JostridDatabase>,
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

    let password_cookie = cookies.get("password");
    let invite = authentication::get_invite(&client, password_cookie).await?;

    Ok(Template::render(
        resolved.clone(),
        MainContext {
            invite: invite,
            route: resolved.clone(),
        },
    ))
}

impl Controller for TemplateController {
    fn get_routes(&self) -> Vec<Route> {
        routes![get_template, login]
    }

    fn get_basepath(&self) -> &str {
        "/"
    }
}
