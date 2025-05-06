use async_trait::async_trait;
use uuid::Uuid;
use anyhow::Result;

use crate::domain::entities::user::{User, NewUser, UserLogin, AuthToken};

#[async_trait]
pub trait UserRegistrationUseCase: Send + Sync {
    async fn register_user(&self, new_user: NewUser) -> Result<User>;
}

#[async_trait]
pub trait UserAuthenticationUseCase: Send + Sync {
    async fn login(&self, credentials: UserLogin) -> Result<AuthToken>;
    async fn validate_token(&self, token: &str) -> Result<bool>;
}

#[async_trait]
pub trait UserProfileUseCase: Send + Sync {
    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<User>>;
    async fn update_user_profile(&self, user_id: Uuid, user_data: User) -> Result<User>;
} 