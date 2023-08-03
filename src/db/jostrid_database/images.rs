use rocket::{
    futures::{StreamExt, TryStreamExt},
    serde::{Deserialize, Serialize},
};
use rocket_db_pools::{
    mongodb::{
        bson::{doc, DateTime},
        options::FindOptions,
    },
    Connection,
};

use crate::error::Error;

use super::JostridDatabase;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Image {
    pub url: String,
    pub created: DateTime,
    pub width: u32,
    pub height: u32,
    pub session_id: String,
}

fn get_collection(
    client: &Connection<JostridDatabase>,
) -> rocket_db_pools::mongodb::Collection<Image> {
    client.database("jostrid").collection::<Image>("images")
}

#[async_trait]
pub trait ImagesDb {
    async fn remove_image(&self, url: String, session_id: &str) -> Result<(), Error>;
    async fn add_image(
        &self,
        url: String,
        width: u32,
        height: u32,
        session_id: &str,
    ) -> Result<(), Error>;
    async fn get_images(&self) -> Result<Vec<Image>, Error>;
}

#[async_trait]
impl ImagesDb for Connection<JostridDatabase> {
    async fn remove_image(&self, url: String, session_id: &str) -> Result<(), Error> {
        get_collection(self)
            .delete_one(
                doc! {
                    "url": url,
                    "session_id": session_id
                },
                None,
            )
            .await?;

        Ok(())
    }

    async fn add_image(
        &self,
        url: String,
        width: u32,
        height: u32,
        session_id: &str,
    ) -> Result<(), Error> {
        get_collection(self)
            .insert_one(
                Image {
                    url,
                    created: DateTime::now(),
                    width,
                    height,
                    session_id: session_id.to_owned(),
                },
                None,
            )
            .await?;

        Ok(())
    }
    async fn get_images(&self) -> Result<Vec<Image>, Error> {
        let results = get_collection(self)
            .find(
                doc! {},
                FindOptions::builder().sort(doc! { "created": -1 }).build(),
            )
            .await?
            .try_collect()
            .await?;

        Ok(results)
    }
}
