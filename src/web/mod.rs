//! Local-first HTTP application and versioned JSON API.

mod error;
mod forms;
mod routes;
mod views;

use std::sync::Arc;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::astro::ChartCalculator;
use crate::locations::CityIndex;
use crate::store::Store;

pub use error::WebError;
pub use routes::router;

#[derive(Debug, Clone)]
pub struct AppState {
    pub calculator: Arc<ChartCalculator>,
    pub store: Store,
    pub cities: CityIndex,
}

impl AppState {
    #[must_use]
    pub fn new(calculator: ChartCalculator, store: Store, cities: CityIndex) -> Self {
        Self {
            calculator: Arc::new(calculator),
            store,
            cities,
        }
    }
}

#[must_use]
pub fn app(state: AppState) -> Router {
    router(state)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(TraceLayer::new_for_http())
}
