use actix_web::{web, HttpResponse, Responder};
use actix_files::Files;

pub async fn landing_index() -> impl Responder {
    HttpResponse::Ok().body("Welcome to R4GMI")
}

pub fn landing_static_files(dist_path: &str) -> Files {
    Files::new("/", dist_path)
        .index_file("index.html")
        .prefer_utf8(true)
} 