#[macro_use]
extern crate rocket;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::Template;
use std::ops::Deref;

use rocket_db_pools::mongodb::{
    bson::doc,
    Client,
};
use rocket_db_pools::{Connection, Database};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct IndexContext;

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
fn index() -> Template {
    Template::render("index", IndexContext)
}

#[derive(Database)]
#[database("jostrid")]
pub struct Db(Client);

#[post("/", data = "<request>")]
async fn login<'r>(
    request: Form<LoginRequest<'r>>,
    client: Connection<Db>,
) -> Result<Json<Invite>, Status> {
    let invite: Invite = match client
        .deref()
        .database("jostrid")
        .collection::<Invite>("invites")
        .find_one(doc! { "password": request.password }, None)
        .await
    {
        Ok(invite) => invite.ok_or(Status::NotFound),
        Err(e) => {
            println!("Failed to fetch {}", e);
            Err(Status::InternalServerError)
        }
    }?;

    Ok(Json(invite))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::init())
        .mount("/", routes![index, login])
        .mount("/public", FileServer::from("./public"))
        .attach(Template::fairing())
}
