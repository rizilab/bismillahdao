use crate::app::App;
use futures_signals::signal::Mutable;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub enum LoginStage {
    Email,
    Password,
    ForgotPassword,
}

#[derive(Clone)]
pub struct LoginState {
    pub app: Arc<App>,
    pub email: Mutable<String>,
    pub password: Mutable<String>,
    pub email_editable: Mutable<bool>,
    pub stage: Mutable<LoginStage>,
    pub title: Mutable<String>,
    pub description: Mutable<String>,
}

impl LoginState {
    pub fn new(app: Arc<App>) -> Arc<Self> {
        Arc::new(Self {
            app,
            email: Mutable::new(String::new()),
            password: Mutable::new(String::new()),
            email_editable: Mutable::new(true),
            stage: Mutable::new(LoginStage::Email),
            title: Mutable::new("Sign in".to_string()),
            description: Mutable::new("Do you have R4GMI account? We recommend signing in using your email address.".to_string()),
        })
    }

    pub fn update_title_and_description(&self) {
        match self.stage.get() {
            LoginStage::Email => {
                self.title.set_neq("Sign in".to_string());
                self.description.set_neq("Do you have R4GMI account? We recommend signing in using your email address.".to_string());
            }
            LoginStage::Password => {
                self.title.set_neq("Enter Your Password".to_string());
                self.description.set_neq("Enter your password for R4GMI to continue using the application.".to_string());
            }
            LoginStage::ForgotPassword => {
                self.title.set_neq("Forgot Your Password?".to_string());
                self.description.set_neq("Enter your email address and we will send you instructions to reset your password.".to_string());
            }
        }
    }

    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }

    pub fn has_minimum_length(password: &str) -> bool {
        password.len() >= 8
    }

    pub fn has_number(password: &str) -> bool {
        password.chars().any(|c| c.is_numeric())
    }

    pub fn has_symbol(password: &str) -> bool {
        password.chars().any(|c| !c.is_alphanumeric())
    }

    pub fn reset_state(&self) {
        self.stage.set_neq(LoginStage::Email);
        self.password.set_neq(String::new());
        self.email_editable.set_neq(true);
        self.email.set_neq(String::new());
        self.update_title_and_description();
    }
}