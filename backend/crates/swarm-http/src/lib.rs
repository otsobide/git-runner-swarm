//! The HTTP API runners are managed through: the adapter that drives the
//! application from the web panel, or from anything else that speaks
//! JSON.

mod api;
mod dto;
mod error;

use std::sync::Arc;

use axum::Router;
use swarm_application::RunnerService;

pub use api::api;

/// The panel's HTTP server: the API under `/api`.
pub fn router(service: Arc<RunnerService>) -> Router {
    Router::new().nest("/api", api(service))
}
