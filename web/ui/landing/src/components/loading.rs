use dominator::{html, Dom};
use futures_signals::signal::{Signal, SignalExt};

pub fn loading_indicator<S>(is_loading: S) -> Dom 
where
    S: Signal<Item = bool> + 'static
{
    html!("div", {
        .class("loading loading-spinner loading-lg")
        .class_signal("hidden", is_loading.map(|loading| !loading))
    })
}