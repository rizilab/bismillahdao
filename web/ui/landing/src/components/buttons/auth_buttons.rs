use dominator::{html, Dom};

pub struct AuthButtons;

impl AuthButtons {
    pub fn render() -> Dom {
        html!("div", {
            .class(["space-y-3"])
            .children(&mut [
                // Google button
                html!("button", {
                    .class(["btn", "btn-outline", "w-full", "gap-2"])
                    .children(&mut [
                        html!("img", {
                            .class(["w-5", "h-5"])
                            .attribute("src", "/public/google-icon.svg")
                            .attribute("alt", "Google")
                        }),
                        html!("span", {
                            .text("Continue with Google")
                        })
                    ])
                }),
                
                // GitHub button
                html!("button", {
                    .class(["btn", "btn-outline", "w-full", "gap-2"])
                    .children(&mut [
                        html!("img", {
                            .class(["w-5", "h-5"])
                            .attribute("src", "/public/github-mark-white.svg")
                            .attribute("alt", "GitHub")
                        }),
                        html!("span", {
                            .text("Continue with GitHub")
                        })
                    ])
                })
            ])
        })
    }
}