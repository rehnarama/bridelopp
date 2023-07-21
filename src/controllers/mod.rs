use rocket::{Build, Rocket};

use crate::lib::Controller;

mod admin_controller;
mod azure_oauth_controller;
mod email_controller;
mod public_controller;
mod registration_controller;
mod spotify_oauth_controller;
pub mod template_controller;
pub mod music_controller;

pub fn mount(mut builder: Rocket<Build>) -> Rocket<Build> {
    let public_controller = public_controller::PublicController {};
    let template_controller = template_controller::TemplateController {};
    let admin_controller = admin_controller::AdminController {};
    let azure_oauth_controller = azure_oauth_controller::AzureOauthController {};
    let email_controller = email_controller::IncomingEmailController {};
    let registration_controller = registration_controller::RegistrationController {};
    let spotify_controller = spotify_oauth_controller::SpotifyOauthController {};
    let music_controller = music_controller::MusicController {};

    builder = public_controller.mount(builder);
    builder = template_controller.mount(builder);
    builder = admin_controller.mount(builder);
    builder = azure_oauth_controller.mount(builder);
    builder = email_controller.mount(builder);
    builder = registration_controller.mount(builder);
    builder = spotify_controller.mount(builder);
    builder = music_controller.mount(builder);

    builder
}
