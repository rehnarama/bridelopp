#[macro_use]
extern crate rocket;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::Template;
use std::ops::Deref;

use rocket_db_pools::mongodb::{bson::doc, Client};
use rocket_db_pools::{Connection, Database};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LoginContext;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct MainContext {
    invite: Invite,
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

#[get("/")]
async fn index(client: Connection<Db>, cookies: &CookieJar<'_>) -> Result<Template, Status> {
    match cookies
        .get("password") {
            Some(password) => match get_invite(client, password.value().to_string()).await? {
                Some(invite) => Ok(Template::render("main", MainContext { invite })),
                None => Err(Status::InternalServerError),
            },
            None => Ok(Template::render("login", LoginContext))
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

    Ok(Redirect::to(uri!(index)))
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
        .mount("/", routes![index, login])
        .mount("/public", FileServer::from("./public"))
        .attach(Template::fairing())
}
