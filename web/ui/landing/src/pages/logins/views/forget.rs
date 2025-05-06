use crate::pages::logins::state::LoginState;
use crate::pages::logins::state::LoginStage;

use std::sync::Arc;
use web_sys::HtmlInputElement;

use dominator::{html, Dom, clone, events, with_node};

pub struct ForgetView;
impl ForgetView {
    pub fn render(state: Arc<LoginState>) -> Dom {
        html!("div", {
            .class(["form-control", "w-full"])
            .children(&mut [
                html!("label", {
                    .class(["label"])
                    .children(&mut [
                        html!("span", {
                            .class(["label-text"])
                            .text("Email address:")
                        })
                    ])
                }),
                html!("input" => HtmlInputElement, {
                    .class(["input", "input-bordered", "w-full"])
                    .class_signal("input-error", state.email.signal_ref(|email| !email.is_empty() && !LoginState::is_valid_email(email)))
                    .attribute("type", "email")
                    .attribute("placeholder", "Enter your email")
                    .property_signal("value", state.email.signal_cloned())
                    .with_node!(input => {
                        .event(clone!(state => move |_: events::Input| {
                            state.email.set_neq(input.value());
                        }))
                    })
                }),
                html!("button", {
                    .class(["btn", "btn-primary", "w-full", "mt-4"])
                    .text("Reset password")
                    .attribute_signal("disabled", state.email.signal_ref(|email| {
                        if email.is_empty() || !LoginState::is_valid_email(email) {
                            Some("true")
                        } else {
                            None
                        }
                    }))
                }),
                html!("div", {
                    .class(["text-center", "mt-4"])
                    .children(&mut [
                        html!("a", {
                            .class(["link", "link-primary", "text-sm"])
                            .text("Back to login")
                            .event(clone!(state => move |_: events::Click| {
                                state.stage.set_neq(LoginStage::Email);
                                state.update_title_and_description();
                            }))
                        })
                    ])
                })
            ])
        })
    }
}