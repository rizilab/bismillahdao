use actix_web::{web, HttpResponse, Responder};
use actix_files::Files;

pub async fn auth_index() -> impl Responder {
    HttpResponse::Ok().body("Auth Service")
}

pub fn auth_static_files(dist_path: &str) -> Files {
    Files::new("/", dist_path)
        .index_file("index.html")
        .prefer_utf8(true)
} 