use actix_web::{web, Scope};

use super::handlers::{register_user, login_user};
 
pub fn user_routes() -> Scope {
    web::scope("/api/users")
        .route("/register", web::post().to(register_user::<crate::application::services::UserService<crate::infrastructure::driven::database::PostgresUserRepository>>))
        .route("/login", web::post().to(login_user::<crate::application::services::UserService<crate::infrastructure::driven::database::PostgresUserRepository>>))
} 