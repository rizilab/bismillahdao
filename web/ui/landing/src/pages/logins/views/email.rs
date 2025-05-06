use dominator::{html, Dom, clone, events, with_node};
use web_sys::HtmlInputElement;
use crate::app::App;
use crate::pages::logins::state::{LoginState, LoginStage};
use std::sync::Arc;
use crate::components::{
    form::EmailInput,
    socials_auth::SocialsAuth
};

pub struct EmailView;

impl EmailView {
    pub fn render(state: Arc<LoginState>) -> Dom {
        html!("div", {
            .children(&mut [
                EmailInput::new(
                    state.email.clone(),
                    Some(Box::new(clone!(state => move |new_value| {
                        state.email.set_neq(new_value);
                    }))),
                    Some(Box::new(clone!(state => move || {
                        if LoginState::is_valid_email(&state.email.get_cloned()) {
                            state.stage.set_neq(LoginStage::Password);
                            state.update_title_and_description();
                        }
                    })))
                ).render(),
                html!("button", {
                    .class(["btn", "btn-primary", "w-full", "mt-4"])
                    .text("Continue")
                    .attribute_signal("disabled", state.email.signal_ref(|email| {
                        if email.is_empty() || !LoginState::is_valid_email(email) {
                            Some("true")
                        } else {
                            None
                        }
                    }))
                    .event(clone!(state => move |_: events::Click| {
                        if LoginState::is_valid_email(&state.email.get_cloned()) {
                            state.stage.set_neq(LoginStage::Password);
                            state.update_title_and_description();
                        }
                    }))
                }),
                SocialsAuth {
                    app: state.app.clone(),
                    text: "Don't have an account? ",
                    link_text: "Sign up",
                    link_route: "/signup",
                }.render(),
            ])
        })
    }
}