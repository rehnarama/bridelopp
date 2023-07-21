use async_trait::async_trait;
use std::ops::Deref;

use rocket::{
    futures::{StreamExt, TryStreamExt},
    http::Status,
    serde::{Deserialize, Serialize},
};
use rocket_db_pools::{
    mongodb::{
        bson::{doc, from_document, Bson, DateTime},
        options::UpdateOptions,
    },
    Connection,
};

use crate::controllers::music_controller::VoteDto;

use super::JostridDatabase;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct SpotifyUser {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: DateTime,
    pub queue: Vec<QueueItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct QueueItem {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub image: String,
    pub votes: u32,
}

impl Into<Bson> for QueueItem {
    fn into(self) -> Bson {
        todo!()
    }
}

fn get_collection(
    client: &Connection<JostridDatabase>,
) -> rocket_db_pools::mongodb::Collection<SpotifyUser> {
    client
        .deref()
        .database("jostrid")
        .collection::<SpotifyUser>("spotify_user")
}

#[async_trait]
pub trait SpotifyDb {
    async fn get_queue_item(
        &self,
        uri: String,
    ) -> Result<Option<QueueItem>, rocket_db_pools::mongodb::error::Error>;
    async fn add_vote(
        &self,
        vote: VoteDto,
    ) -> Result<(), rocket_db_pools::mongodb::error::Error>;
    async fn get_user(&self)
        -> Result<Option<SpotifyUser>, rocket_db_pools::mongodb::error::Error>;
    async fn insert_user(
        &self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    ) -> Result<SpotifyUser, rocket_db_pools::mongodb::error::Error>;
    async fn refresh_user(
        &self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    ) -> Result<SpotifyUser, rocket_db_pools::mongodb::error::Error>;
}

#[async_trait]
impl SpotifyDb for Connection<JostridDatabase> {
    async fn get_queue_item(
        &self,
        uri: String,
    ) -> Result<Option<QueueItem>, rocket_db_pools::mongodb::error::Error> {
        let mut cursor = get_collection(self)
            .aggregate(
                [
                    doc! { "$unwind": "$queue" },
                    doc! { "$match": { "queue.uri": uri } },
                    doc! { "$project": { "queue": true } },
                    doc! { "$replaceRoot": { "newRoot": "$queue" }},
                ],
                None,
            )
            .await?;

        Ok(cursor
            .try_next()
            .await?
            .map(|doc| from_document::<QueueItem>(doc).unwrap()))
    }

    async fn add_vote(
        &self,
        vote: VoteDto,
    ) -> Result<(), rocket_db_pools::mongodb::error::Error> {
        match self.get_queue_item(vote.uri.to_owned()).await? {
            Some(vote) => {
                get_collection(self)
                    .update_one(
                        doc! { "queue.uri": vote.uri },
                        doc! { "$inc": { "queue.$.votes": 1 } },
                        None,
                    )
                    .await?
            }
            None => {
                get_collection(self)
                    .update_one(
                        doc! {},
                        doc! {
                            "$push": { "queue": {
                                "uri": vote.uri.to_owned(),
                                "name": vote.name,
                                "artist": vote.artist,
                                "image": vote.image,
                                "votes": 1
                            }}
                        },
                        None,
                    )
                    .await?
            }
        };

        Ok(())
    }

    async fn get_user(
        &self,
    ) -> Result<Option<SpotifyUser>, rocket_db_pools::mongodb::error::Error> {
        Ok(get_collection(self).find_one(doc! {}, None).await?)
    }

    async fn insert_user(
        &self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    ) -> Result<SpotifyUser, rocket_db_pools::mongodb::error::Error> {
        let queue: Vec<QueueItem> = Vec::new();
        get_collection(self)
            .update_one(
                doc! {},
                doc! {
                    "$set": {
                        "access_token": access_token,
                        "refresh_token": refresh_token,
                        "expires": DateTime::from_millis(DateTime::now().timestamp_millis() + expires_in * 1000),
                        "queue": queue
                    }
                },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(self.get_user().await?.unwrap())
    }

    async fn refresh_user(
        &self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    ) -> Result<SpotifyUser, rocket_db_pools::mongodb::error::Error> {
        get_collection(self)
            .update_one(
                doc! {},
                doc! {
                    "$set": {
                        "access_token": access_token,
                        "refresh_token": refresh_token,
                        "expires": DateTime::from_millis(DateTime::now().timestamp_millis() + expires_in * 1000)
                    }
                },
                None
            )
            .await?;

        Ok(self.get_user().await?.unwrap())
    }
}
