use std::ops::Deref;

use rocket::{
    http::Status,
    serde::{Deserialize, Serialize},
};
use rocket_db_pools::{mongodb::bson::doc, Connection};

use super::JostridDatabase;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Invite {
    password: String,
    name: String,
}

pub async fn get_invite(
    client: Connection<JostridDatabase>,
    password: String,
) -> Result<Option<Invite>, Status> {
    client
        .deref()
        .database("jostrid")
        .collection::<Invite>("invites")
        .find_one(doc! { "password": password }, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })
}
