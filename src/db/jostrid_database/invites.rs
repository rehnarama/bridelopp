use std::ops::Deref;

use rocket::{
    http::Status,
    serde::{Deserialize, Serialize},
};
use rocket_db_pools::{
    mongodb::bson::{doc, Bson},
    Connection,
};

use super::JostridDatabase;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Response {
    pub name: String,
    pub attending: bool,
    pub food_preferences: Option<String>,
}

impl Into<Bson> for Response {
    fn into(self) -> Bson {
        Bson::Document(doc! {
            "name": self.name,
            "attending": self.attending,
            "food_preferences": self.food_preferences.unwrap_or("".to_string())
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Invite {
    pub password: String,
    pub name: String,
    pub responses: Vec<Response>,
    pub plus_one: bool
}

fn get_collection(
    client: &Connection<JostridDatabase>,
) -> rocket_db_pools::mongodb::Collection<Invite> {
    client
        .deref()
        .database("jostrid")
        .collection::<Invite>("invites")
}

pub async fn get_invite(
    client: &Connection<JostridDatabase>,
    password: String,
) -> Result<Option<Invite>, Status> {
    get_collection(client)
        .find_one(doc! { "password": password }, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })
}

pub async fn get_invite_by_id(
    client: &Connection<JostridDatabase>,
    id: Bson,
) -> Result<Option<Invite>, Status> {
    get_collection(client)
        .find_one(doc! { "_id": id }, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })
}

pub async fn add_invite(
    client: &Connection<JostridDatabase>,
    invite: Invite,
) -> Result<(), Status> {
    get_collection(client)
        .insert_one(invite, None)
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })?;

    Ok(())
}

pub async fn add_responses(
    client: &Connection<JostridDatabase>,
    password: &str,
    responses: Vec<Response>,
    plus_one: bool
) -> Result<(), Status> {
    get_collection(client)
        .update_one(
            doc! { "password": password },
            doc! {
                "$set": {
                    "responses": responses,
                    "plus_one": plus_one
                }
            },
            None,
        )
        .await
        .map_err(|e| {
            error!("Failed {}", e);
            Status::InternalServerError
        })?;

    Ok(())
}
