use rocket::{figment::Figment, Build, Rocket};

use crate::lib::Controller;

mod admin_controller;
mod azure_oauth_controller;
mod email_controller;
pub mod music_controller;
mod public_controller;
mod registration_controller;
mod spotify_oauth_controller;
pub mod template_controller;

pub fn mount(mut builder: Rocket<Build>, config: Figment) -> Rocket<Build> {
    let public_controller = public_controller::PublicController {};
    let template_controller = template_controller::TemplateController {};
    let admin_controller = admin_controller::AdminController {};
    let azure_oauth_controller = azure_oauth_controller::AzureOauthController {};
    let email_controller = email_controller::IncomingEmailController {};
    let registration_controller = registration_controller::RegistrationController {};
    let spotify_controller = spotify_oauth_controller::SpotifyOauthController {};

    let address = config.find_value("address").unwrap();
    let port = config.find_value("port").unwrap();

    let music_controller = music_controller::MusicController::new(
        address.as_str().unwrap().to_owned(),
        port.to_u128().unwrap().to_string(),
    );

    builder = public_controller.mount(builder, &config);
    builder = template_controller.mount(builder, &config);
    builder = admin_controller.mount(builder, &config);
    builder = azure_oauth_controller.mount(builder, &config);
    builder = email_controller.mount(builder, &config);
    builder = registration_controller.mount(builder, &config);
    builder = spotify_controller.mount(builder, &config);
    builder = music_controller.mount(builder, &config);

    builder = builder.manage(music_controller);

    builder
}
