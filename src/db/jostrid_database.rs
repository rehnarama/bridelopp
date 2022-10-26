use rocket_db_pools::{Database, mongodb::Client};

pub mod invites;
pub mod responses;

#[derive(Database)]
#[database("jostrid")]
pub struct JostridDatabase(Client);
