use std::ops::Deref;

use rocket::{
    http::Status,
    serde::{Deserialize, Serialize}, futures::{StreamExt, TryStreamExt},
};
use rocket_db_pools::{
    mongodb::bson::{doc, Bson},
    Connection,
};

use super::JostridDatabase;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Response {
    pub password: String,
    pub name: String,
}

fn get_collection(
    client: &Connection<JostridDatabase>,
) -> rocket_db_pools::mongodb::Collection<Response> {
    client
        .deref()
        .database("jostrid")
        .collection::<Response>("responses")
}

pub async fn get_responses(
    client: &Connection<JostridDatabase>,
    password: String,
) -> Result<Vec<Response>, Status> {
    let cursor =
        get_collection(client)
        .find(doc! { "password": password }, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })?;

    Ok(cursor.try_collect().await.unwrap_or_else(|_| vec![]))
}

pub async fn add_responses(client: &Connection<JostridDatabase>, responses: Vec<Response>) -> Result<(), Status> {
    get_collection(client)
        .insert_many(responses, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })?;

    Ok(())
}

