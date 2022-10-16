use rocket::{serde::{Serialize, Deserialize}, http::Status};

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "camelCase")]
pub struct MeResponse {
    pub display_name: String,
    pub user_principal_name: String,
}

pub async fn get_user(bearer: String) -> Result<MeResponse, Status> {
    let url = "https://graph.microsoft.com/v1.0/me";

    let client = reqwest::Client::new();
    let me_response = match client.get(url).bearer_auth(bearer).send().await {
        Ok(response) => match response.json::<MeResponse>().await {
            Ok(body) => Ok(body),
            Err(e) => {
                error!("Error: {}", e);
                Err(Status::InternalServerError)
            }
        },
        Err(e) => {
            error!("Error: {}", e);
            Err(Status::InternalServerError)
        }
    }?;

    let allowed_emails = vec!["astrid.rehn@outlook.com", "josefin.ahlenius@hotmail.com"];

    let email: &str = &me_response.user_principal_name;
    if allowed_emails.contains(&email) {
        Ok(me_response)
    } else {
        Err(Status::Forbidden)
    }
}