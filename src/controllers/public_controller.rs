use std::path::{Path, PathBuf};

use rocket::{fs::NamedFile, http::Header, Route};

use crate::lib::Controller;

pub struct PublicController;

#[derive(Responder)]
struct StaticFileResponder {
    inner: NamedFile,
    header: Header<'static>,
}

#[get("/public/<file..>", rank = 1)]
async fn public(file: PathBuf) -> Option<StaticFileResponder> {
    NamedFile::open(Path::new("public/").join(file))
        .await
        .ok()
        .map(|named_file| StaticFileResponder {
            inner: named_file,
            header: Header::new("Cache-Control", "max-age=3600"),
        })
}

impl Controller for PublicController {
    fn get_routes(&self) -> Vec<Route> {
        routes![public]
    }

    fn get_basepath(&self) -> &str {
        "/"
    }
}
