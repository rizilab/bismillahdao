use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use anyhow::{anyhow, Result};

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();
            
        Ok(password_hash)
    }
    
    pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|e| anyhow!("Failed to parse hash: {}", e))?;
            
        let result = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map(|_| true)
            .or_else(|_| Ok(false))?;
            
        Ok(result)
    }
} 