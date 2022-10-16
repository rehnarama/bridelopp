
use rocket::{Route};

use crate::lib::Controller;

pub struct IncomingEmailController;

#[post("/", rank = 2, data = "<body>")]
fn incoming_email(body: String) -> &'static str {
    info!("Got incoming email: {}", body);

    "OK"
}

impl Controller for IncomingEmailController {
    fn get_routes(&self) -> Vec<Route> {
        routes![incoming_email]
    }

    fn get_basepath(&self) -> &str {
        "/incoming-email"
    }
}
