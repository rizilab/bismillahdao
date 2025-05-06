use std::sync::Arc;
use dominator::routing;
use futures_signals::signal::{Mutable, SignalExt};
use log;
use wasm_bindgen_futures::spawn_local;
use url;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    Login,
    Signup,
    NotFound,
}

pub struct Router {
    pub current_route: Mutable<Route>,
}

impl Router {
    pub fn new() -> Arc<Self> {
        let router = Arc::new(Self {
            current_route: Mutable::new(Route::Login),
        });

        // Set up route handling
        {
            let router_ref = router.clone();
            let router_ref2 = router.clone();
            
            // Subscribe to URL changes and spawn the future
            spawn_local(async move {
                routing::url()
                    .signal_ref(move |url| {
                        log::debug!("URL signal received: {}", url);
                        router_ref.url_to_route(url)
                    })
                    .for_each(move |route| {
                        log::debug!("Route changed to: {:?}", route);
                        router_ref2.current_route.set(route);
                        async {}
                    })
                    .await;
            });
        }

        router
    }

    pub fn push(&self, path: &str) {
        log::debug!("Pushing route: {}", path);
        let current = self.current_route.get();
        log::debug!("Current route before push: {:?}", current);
        
        routing::go_to_url(path);
        
        // Verify route change
        let new_route = self.url_to_route(path);
        log::debug!("New route after push: {:?}", new_route);
        self.current_route.set(new_route);
    }

    fn url_to_route(&self, url: &str) -> Route {
        let path = if let Ok(parsed_url) = url::Url::parse(url) {
            parsed_url.path().to_string()
        } else {
            url.split('?').next().unwrap_or(url).to_string()
        };

        log::debug!("Parsed path: {}", path);
        
        let route = match path.as_str() {
            "/" => Route::Login,
            "/signup" => Route::Signup,
            _ => {
                log::debug!("No route match for path: {}", path);
                Route::NotFound
            }
        };
        
        log::debug!("Mapped URL {} to route {:?}", url, route);
        route
    }
} 