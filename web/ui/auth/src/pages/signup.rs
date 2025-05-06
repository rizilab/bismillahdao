use dominator::{html, Dom, clone, events};
use std::sync::Arc;
use futures_signals::signal::Mutable;
use crate::app::App;
use crate::adapters::primary::ui::components::form::EmailInput;
use crate::adapters::primary::ui::components::form::UsernameInput; 
use crate::adapters::primary::ui::components::socials_auth::SocialsAuth;

#[derive(Clone)]
pub struct SignupPage {
    app: Arc<App>,
    email: Mutable<String>,
    username: Mutable<String>,
}

impl SignupPage {
    pub fn new(app: Arc<App>) -> Arc<Self> {
        Arc::new(Self {
            app,
            email: Mutable::new(String::new()),
            username: Mutable::new(String::new()),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        html!("div", {
            .class([
                "min-h-screen", 
                "bg-base-200", 
                "flex", 
                "flex-col"
            ])
            .children(&mut [
                html!("div", {
                    .class([
                        "flex-1",
                        "flex",
                        "flex-col",
                        "items-center",
                        "justify-center",
                        "p-4",
                    ])
                    .children(&mut [
                        html!("div", {
                            .class(["card", "w-full", "max-w-md", "bg-base-100", "shadow-xl"])
                            .children(&mut [
                                html!("div", {
                                    .class(["card-body"])
                                    .children(&mut [
                                        html!("h2", {
                                            .class(["text-2xl", "font-bold", "text-center", "mb-2", "w-full"])
                                            .text("Create your account")
                                        }),
                                        EmailInput::new(self.email.clone(), None, None).render(),
                                        UsernameInput::new(self.username.clone(), None).render(),
                                        // Newsletter checkbox
                                        html!("div", {
                                            .class(["form-control", "mt-6"])
                                            .children(&mut [
                                                html!("label", {
                                                    .class(["label", "cursor-pointer", "justify-start", "gap-2"])
                                                    .children(&mut [
                                                        html!("input", {
                                                            .class(["checkbox", "checkbox-sm"])
                                                            .attribute("type", "checkbox")
                                                        }),
                                                        html!("span", {
                                                            .class(["label-text"])
                                                            .text("Send me occasional product updates and announcements.")
                                                        })
                                                    ])
                                                })
                                            ])
                                        }),
                                        // Sign up button
                                        html!("button", {
                                            .class(["btn", "btn-primary", "w-full", "mt-6"])
                                            .text("Sign up")
                                        }),
                                        SocialsAuth {
                                            app: self.app.clone(),
                                            text: "Already have an account? ",
                                            link_text: "Sign in",
                                            link_route: "/",
                                        }.render(),
                                    ])
                                })
                            ])
                        })
                    ])
                })
            ])
        })
    }
} 