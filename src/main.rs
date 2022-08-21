#[macro_use]
extern crate rocket;
use rocket::form::Form;
use rocket::fs::{FileServer, NamedFile};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::Template;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use rocket_db_pools::mongodb::{bson::doc, Client};
use rocket_db_pools::{Connection, Database};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LoginContext;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct MainContext {
    invite: Invite,
    route: String,
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

#[get("/<path..>", rank = 2)]
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
        .mount("/", routes![public, index, login])
        .attach(Template::fairing())
}
