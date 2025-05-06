use dominator::{html, Dom, clone, events};
use std::sync::Arc;
use crate::app::App;
use super::buttons::AuthButtons;

pub struct SocialsAuth {
    pub app: Arc<App>,
    pub text: &'static str,
    pub link_text: &'static str,
    pub link_route: &'static str,
}

impl SocialsAuth {
    pub fn render(&self) -> Dom {
        let app = self.app.clone();
        let link_route = self.link_route.to_string();
        
        html!("div", {
            .class(["mt-6"])
            .children(&mut [
                html!("div", {
                    .class(["divider"])
                    .text("OR")
                }),
                AuthButtons::render(),
                html!("p", {
                    .class(["text-sm", "text-center", "mt-6"])
                    .children(&mut [
                        html!("span", {
                            .text(self.text)
                        }),
                        html!("a", {
                            .class(["link", "link-primary"])
                            .text(self.link_text)
                            .event(clone!(app, link_route => move |_: events::Click| {
                                app.router.push(&link_route);
                            }))
                        })
                    ])
                })
            ])
        })
    }
} 