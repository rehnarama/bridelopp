use std::path::{Path, PathBuf};

use rocket::{fs::NamedFile, Route};

use crate::lib::Controller;

pub struct PublicController;

#[get("/public/<file..>", rank = 1)]
async fn public(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("public/").join(file)).await.ok()
}

impl Controller for PublicController {
    fn get_routes(&self) -> Vec<Route> {
        routes![public]
    }

    fn get_basepath(&self) -> &str {
        "/"
    }
}
