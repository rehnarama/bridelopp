use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AzureConfig {
    pub secret: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SpotifyConfig {
    pub secret: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AppConfig {
    pub azure: AzureConfig,
    pub spotify: SpotifyConfig,
}
