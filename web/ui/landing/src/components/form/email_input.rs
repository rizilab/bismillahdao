use dominator::{html, Dom, clone, events, with_node};
use web_sys::HtmlInputElement;
use std::sync::Arc;
use futures_signals::signal::{Mutable, SignalExt, Signal};

pub struct EmailInput {
    pub email: Mutable<String>,
    pub on_change: Option<Box<dyn Fn(String)>>,
    pub on_enter: Option<Box<dyn Fn()>>,
}

impl EmailInput {
    pub fn new(
        email: Mutable<String>, 
        on_change: Option<Box<dyn Fn(String)>>,
        on_enter: Option<Box<dyn Fn()>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            email,
            on_change,
            on_enter,
        })
    }
    
    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self.clone();
        let state_for_input = state.clone();
        let state_for_keydown = state.clone();
        let email = state.email.clone();

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
                    .class_signal("input-error", state.email.signal_ref(|email| !email.is_empty() && !Self::is_valid_email(email)))
                    .attribute("type", "email")
                    .attribute("placeholder", "Enter your email")
                    .property_signal("value", state.email.signal_cloned())
                    .with_node!(input => {
                        .event(move |_: events::Input| {
                            let new_value = input.value();
                            email.set_neq(new_value.clone());
                            if let Some(callback) = &state_for_input.on_change {
                                callback(new_value);
                            }
                        })
                        .event(move |e: events::KeyDown| {
                            if e.key() == "Enter" && Self::is_valid_email(&state_for_keydown.email.get_cloned()) {
                                if let Some(callback) = &state_for_keydown.on_enter {
                                    callback();
                                }
                            }
                        })
                    })
                })
            ])
        })
    }

    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
} 