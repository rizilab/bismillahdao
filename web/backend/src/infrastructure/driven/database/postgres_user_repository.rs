use async_trait::async_trait;
use anyhow::{anyhow, Result};
use sqlx::{PgPool, Pool, Postgres};
use uuid::Uuid;
use std::sync::Arc;

use crate::domain::entities::user::User;
use crate::domain::repositories::user_repository::UserRepository;

pub struct PostgresUserRepository {
    pool: Arc<Pool<Postgres>>,
}

impl PostgresUserRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: &User) -> Result<User> {
        let result = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, email, password_hash, created_at, updated_at
            "#,
            user.id,
            user.username,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result)
    }

    async fn update(&self, user: &User) -> Result<User> {
        let result = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = $1, email = $2, password_hash = $3, updated_at = $4
            WHERE id = $5
            RETURNING id, username, email, password_hash, created_at, updated_at
            "#,
            user.username,
            user.email,
            user.password_hash,
            user.updated_at,
            user.id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(result)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            id
        )
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("User not found"));
        }

        Ok(())
    }
} 