use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use dominator::DomBuilder;
pub use dominator::{clone, events, html, svg, with_node, Dom};
pub use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{MutableVec, SignalVec, SignalVecExt},
};
use web_sys::HtmlInputElement;

use crate::pages::login::LoginPage;
use crate::pages::signup::SignupPage;
use std::error::Error;

use crate::router::{Route, Router};

#[derive(Clone, Serialize, Deserialize)]
struct Args {
    name: String,
}

#[derive(Clone)]
pub struct App {
    pub router: Arc<Router>,
}

impl App {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            router: Router::new(),
        })
    }

    pub fn render(app: Arc<Self>) -> Dom {
        let app_clone = app.clone();
        html!("div", {
            .class(["globa"])
            .child_signal(app_clone.router.current_route.signal_cloned().map(move |route| {
                match route {
                    Route::Login => Some(Arc::new(LoginPage::new(app_clone.clone())).render()),
                    Route::Signup => Some(Arc::new(SignupPage::new(app_clone.clone())).render()),
                    Route::NotFound => None,
                }
            }))
        })
    }
}
