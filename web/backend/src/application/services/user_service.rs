use async_trait::async_trait;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Duration, Utc};

use crate::domain::entities::user::{User, NewUser, UserLogin, AuthToken};
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::services::auth_service::AuthService;
use crate::application::ports::in::{
    UserRegistrationUseCase,
    UserAuthenticationUseCase,
    UserProfileUseCase,
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
    iat: i64,
}

pub struct UserService<R: UserRepository> {
    user_repository: Arc<R>,
    jwt_secret: String,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(user_repository: Arc<R>, jwt_secret: String) -> Self {
        Self {
            user_repository,
            jwt_secret,
        }
    }

    fn generate_token(&self, user_id: Uuid) -> Result<AuthToken> {
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(AuthToken {
            token,
            expires_at,
        })
    }

    fn decode_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}

#[async_trait]
impl<R: UserRepository> UserRegistrationUseCase for UserService<R> {
    async fn register_user(&self, new_user: NewUser) -> Result<User> {
        // Check if user with email already exists
        if let Some(_) = self.user_repository.find_by_email(&new_user.email).await? {
            return Err(anyhow!("User with this email already exists"));
        }

        // Check if username is taken
        if let Some(_) = self.user_repository.find_by_username(&new_user.username).await? {
            return Err(anyhow!("Username is already taken"));
        }

        // Hash the password
        let password_hash = AuthService::hash_password(&new_user.password)?;

        // Create new user with hashed password
        let user = User::new(new_user.username, new_user.email, password_hash);
        
        // Save to repository
        let created_user = self.user_repository.create(&user).await?;
        
        Ok(created_user)
    }
}

#[async_trait]
impl<R: UserRepository> UserAuthenticationUseCase for UserService<R> {
    async fn login(&self, credentials: UserLogin) -> Result<AuthToken> {
        // Find user by email
        let user = match self.user_repository.find_by_email(&credentials.email).await? {
            Some(user) => user,
            None => return Err(anyhow!("Invalid email or password")),
        };

        // Verify password
        let is_valid = AuthService::verify_password(&credentials.password, &user.password_hash)?;
        if !is_valid {
            return Err(anyhow!("Invalid email or password"));
        }

        // Generate JWT token
        let token = self.generate_token(user.id)?;
        
        Ok(token)
    }

    async fn validate_token(&self, token: &str) -> Result<bool> {
        match self.decode_token(token) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl<R: UserRepository> UserProfileUseCase for UserService<R> {
    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<User>> {
        self.user_repository.find_by_id(user_id).await
    }

    async fn update_user_profile(&self, user_id: Uuid, user_data: User) -> Result<User> {
        // First check if user exists
        let existing_user = match self.user_repository.find_by_id(user_id).await? {
            Some(user) => user,
            None => return Err(anyhow!("User not found")),
        };

        // Create updated user with same password if not changed
        let updated_user = User {
            id: existing_user.id,
            username: user_data.username,
            email: user_data.email,
            password_hash: existing_user.password_hash, // Keep existing password
            created_at: existing_user.created_at,
            updated_at: Utc::now(),
        };

        // Update in repository
        let user = self.user_repository.update(&updated_user).await?;
        
        Ok(user)
    }
} 