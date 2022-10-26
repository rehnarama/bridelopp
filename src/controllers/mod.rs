use rocket::{Rocket, Build};

use crate::lib::Controller;


mod public_controller;
mod template_controller;
mod admin_controller;
mod azure_oauth_controller;
mod email_controller;
mod admin_invite_controller;

pub fn mount(mut builder: Rocket<Build>) -> Rocket<Build> {
    let public_controller = public_controller::PublicController {};
    let template_controller = template_controller::TemplateController {};
    let admin_controller = admin_controller::AdminController {};
    let azure_oauth_controller = azure_oauth_controller::AzureOauthController {};
    let email_controller = email_controller::IncomingEmailController {};

    builder = public_controller.mount(builder);
    builder = template_controller.mount(builder);
    builder = admin_controller.mount(builder);
    builder = azure_oauth_controller.mount(builder);
    builder = email_controller.mount(builder);

    builder
}