#[macro_use] extern crate rocket;
use rocket_dyn_templates::Template;
use rocket::serde::{Serialize};
use rocket::fs::{FileServer, relative};
use rocket::form::Form;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct IndexContext;

#[derive(Debug, FromForm)]
struct LoginRequest<'r> {
    password: &'r str,
}

#[get("/")]
fn index() -> Template {
    Template::render("index", IndexContext)
}

#[post("/", data = "<request>")]
fn login<'r>(request: Form<LoginRequest<'r>>) -> &'r str {
    request.password
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, login])
        .mount("/public", FileServer::from(relative!("public")))
        .attach(Template::fairing())
}
