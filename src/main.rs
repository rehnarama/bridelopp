mod config;
mod controllers;
mod db;
mod lib;
mod pkce;

#[macro_use]
extern crate rocket;
extern crate dotenv;

use db::jostrid_database::JostridDatabase;
use dotenv::dotenv;
use rocket::fairing::AdHoc;

use rocket_dyn_templates::Template;

use rocket_db_pools::Database;

use crate::config::AppConfig;

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let mut builder = rocket::build()
        .attach(JostridDatabase::init())
        .attach(Template::fairing())
        .attach(AdHoc::config::<AppConfig>());

    builder = controllers::mount(builder);

    builder
}
