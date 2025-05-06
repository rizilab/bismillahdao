use dominator::{html, Dom, clone};
use std::sync::Arc;
use crate::app::App;
use crate::adapters::primary::ui::pages::logins::state::{LoginState, LoginStage};
use crate::adapters::primary::ui::pages::logins::views::{EmailView, PasswordView, ForgetView};
use futures_signals::signal::SignalExt;

#[derive(Clone)]
pub struct LoginPage {
    state: Arc<LoginState>,
}

impl LoginPage {
    pub fn new(app: Arc<App>) -> Arc<Self> {
        Arc::new(Self { 
            state: LoginState::new(app),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self.state.clone();
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
                            .class(["w-full", "max-w-sm"])
                            .children(&mut [
                                html!("div", {
                                    .class(["text-center"])
                                    .children(&mut [
                                        html!("h2", {
                                            .class(["text-2xl", "font-bold", "mb-2"])
                                            .text_signal(state.title.signal_cloned())
                                        }),
                                        html!("p", {
                                            .class(["text-sm", "text-center", "mb-6", "text-base-content/70"])
                                            .text_signal(state.description.signal_cloned())
                                        }),
                                    ])
                                }),
                                // View selection based on stage
                                html!("div", {
                                    .child_signal(state.stage.signal_cloned().map(clone!(state => move |stage| {
                                        Some(match stage {
                                            LoginStage::Email => EmailView::render(state.clone()),
                                            LoginStage::Password => PasswordView::render(state.clone()),
                                            LoginStage::ForgotPassword => ForgetView::render(state.clone()),
                                        })
                                    })))
                                })
                            ])
                        })
                    ])
                })
            ])
        })
    }
}