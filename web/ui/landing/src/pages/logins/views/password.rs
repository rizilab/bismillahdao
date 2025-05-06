use dominator::{html, Dom, clone, events, with_node};
use web_sys::HtmlInputElement;
use crate::pages::logins::state::{LoginState, LoginStage};
use std::sync::Arc;

pub struct PasswordView;

impl PasswordView {
    pub fn render(state: Arc<LoginState>) -> Dom {
        html!("div", {
            .class(["form-control", "w-full"])
            .children(&mut [
                html!("div", {
                    .class(["flex", "items-center", "justify-between", "mb-6"])
                    .children(&mut [
                        html!("div", {
                            .class(["text-sm", "text-base-content/70"])
                            .text(&format!("Signing in as {}", state.email.get_cloned()))
                        }),
                        html!("button", {
                            .class(["btn", "btn-ghost", "btn-xs"])
                            .text("Edit")
                            .event(clone!(state => move |_: events::Click| {
                                state.stage.set_neq(LoginStage::Email);
                                state.update_title_and_description();
                            }))
                        })
                    ])
                }),
                html!("label", {
                    .class(["label"])
                    .children(&mut [
                        html!("span", {
                            .class(["label-text"])
                            .text("Password:")
                        })
                    ])
                }),
                html!("input" => HtmlInputElement, {
                    .class(["input", "input-bordered", "w-full"])
                    .attribute("type", "password")
                    .attribute("placeholder", "Enter your password")
                    .with_node!(input => {
                        .event(clone!(state => move |_: events::Input| {
                            state.password.set_neq(input.value());
                        }))
                    })
                }),
                // Forgot password link
                html!("div", {
                    .class(["text-right", "mt-2"])
                    .children(&mut [
                        html!("a", {
                            .class(["link", "link-primary", "text-sm"])
                            .text("Forgot password?")
                            .event(clone!(state => move |_: events::Click| {
                                state.stage.set_neq(LoginStage::ForgotPassword);
                                state.password.set_neq(String::new());
                                state.update_title_and_description();
                            }))
                        })
                    ])
                }),
                // Sign in button
                html!("button", {
                    .class(["btn", "btn-primary", "w-full", "mt-4"])
                    .text("Sign in")
                    .with_node!(button => {
                        .event(clone!(state => move |_: events::Click| {
                            if !state.password.get_cloned().is_empty() {
                                log::debug!("Login with email: {} and password", state.email.get_cloned());
                            }
                        }))
                    })
                })
            ])
        })
    }

    fn render_password_requirements(state: Arc<LoginState>) -> Dom {
        html!("div", {
            .class(["mt-2", "space-y-1"])
            .child_signal(state.password.signal_ref(move |password| {
                if !LoginState::has_minimum_length(password) || 
                   !LoginState::has_number(password) || 
                   !LoginState::has_symbol(password) {
                    Some(html!("div", {
                        .class(["space-y-1"])
                        .children(&mut [
                            html!("div", {
                                .class(["text-xs", "text-error"])
                                .text("• Minimum 8 characters")
                                .style("display", if !LoginState::has_minimum_length(password) { "block" } else { "none" })
                            }),
                            html!("div", {
                                .class(["text-xs", "text-error"])
                                .text("• At least one number")
                                .style("display", if !LoginState::has_number(password) { "block" } else { "none" })
                            }),
                            html!("div", {
                                .class(["text-xs", "text-error"])
                                .text("• At least one symbol")
                                .style("display", if !LoginState::has_symbol(password) { "block" } else { "none" })
                            })
                        ])
                    }))
                } else {
                    None
                }
            }))
        })
    }
}