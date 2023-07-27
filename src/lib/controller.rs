use rocket::{Rocket, Build, Route, figment::Figment};

pub trait Controller {
    fn get_basepath(&self) -> &str;

    fn get_routes(&self) -> Vec<Route>;

    fn mount(&self, builder: Rocket<Build>, _config: &Figment) -> Rocket<Build> {
        builder.mount(
            self.get_basepath(),
            self.get_routes()
        )
    }
}