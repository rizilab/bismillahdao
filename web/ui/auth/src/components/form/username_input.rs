use dominator::{html, Dom, clone, events, with_node};
use web_sys::HtmlInputElement;
use std::sync::Arc;
use futures_signals::signal::Mutable;

pub struct UsernameInput {
    pub username: Mutable<String>,
    pub on_change: Option<Box<dyn Fn(String)>>,
}

impl UsernameInput {
    pub fn new(username: Mutable<String>, on_change: Option<Box<dyn Fn(String)>>) -> Arc<Self> {
        Arc::new(Self {
            username,
            on_change,
        })
    }
    
    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self.clone();
        let username = state.username.clone();
        
        html!("div", {
            .class(["form-control", "w-full", "mt-2"])
            .children(&mut [
                html!("label", {
                    .class(["label"])
                    .children(&mut [
                        html!("span", {
                            .class(["label-text"])
                            .text("Username")
                        })
                    ])
                }),
                html!("input" => HtmlInputElement, {
                    .class(["input", "input-bordered", "w-full"])
                    .attribute("type", "text")
                    .attribute("placeholder", "Choose a username")
                    .property_signal("value", state.username.signal_cloned())
                    .with_node!(input => {
                        .event(move |_: events::Input| {
                            let new_value = input.value();
                            username.set_neq(new_value.clone());
                            if let Some(callback) = &state.on_change {
                                callback(new_value);
                            }
                        })
                    })
                })
            ])
        })
    }
} 