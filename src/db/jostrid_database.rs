use rocket::http::Status;
use rocket_db_pools::{Database, mongodb::Client};

pub mod invites;
pub mod spotify;
pub mod images;

#[derive(Database)]
#[database("jostrid")]
pub struct JostridDatabase(Client);
