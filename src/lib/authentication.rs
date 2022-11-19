use rocket::http::{Status, Cookie};
use rocket_db_pools::Connection;

use crate::db::jostrid_database::{
    invites::{self, Invite},
    JostridDatabase,
};

pub async fn get_invite(
    client: &Connection<JostridDatabase>,
    password_cookie: Option<&Cookie<'_>>,
) -> Result<Invite, Status> {
    let password = password_cookie.ok_or(Status::Unauthorized)?;

    invites::get_invite(client, password.value().into())
        .await?
        .ok_or(Status::Unauthorized)
}
