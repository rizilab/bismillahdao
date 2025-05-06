use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::ports::in_ports::{UserRegistrationUseCase, UserAuthenticationUseCase, UserProfileUseCase};
use crate::domain::entities::user::{NewUser, UserLogin};

// AppState containing our application services
pub struct AppState<T: UserRegistrationUseCase + UserAuthenticationUseCase + UserProfileUseCase> {
    pub user_service: Arc<T>,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub username: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// User registration handler
pub async fn register_user<T>(
    data: web::Data<AppState<T>>,
    user_data: web::Json<RegisterRequest>,
) -> impl Responder 
where 
    T: UserRegistrationUseCase + UserAuthenticationUseCase + UserProfileUseCase
{
    let new_user = NewUser {
        username: user_data.username.clone(),
        email: user_data.email.clone(),
        password: user_data.password.clone(),
    };

    match data.user_service.register_user(new_user).await {
        Ok(user) => {
            let response = RegisterResponse {
                id: user.id.to_string(),
                username: user.username,
                email: user.email,
            };
            HttpResponse::Created().json(response)
        },
        Err(e) => {
            // Handle specific error types more elegantly in production
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

// User login handler
pub async fn login_user<T>(
    data: web::Data<AppState<T>>,
    credentials: web::Json<LoginRequest>,
) -> impl Responder 
where 
    T: UserRegistrationUseCase + UserAuthenticationUseCase + UserProfileUseCase
{
    let login = UserLogin {
        email: credentials.email.clone(),
        password: credentials.password.clone(),
    };

    match data.user_service.login(login).await {
        Ok(token) => {
            let response = LoginResponse {
                token: token.token,
            };
            HttpResponse::Ok().json(response)
        },
        Err(e) => {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
} 