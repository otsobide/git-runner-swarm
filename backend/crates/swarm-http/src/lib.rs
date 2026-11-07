//! The HTTP API runners are managed through: the adapter that drives the
//! application from the web panel, or from anything else that speaks
//! JSON. It serves the panel's own files too, so one server, and one
//! container, is the whole panel.

mod api;
mod dto;
mod error;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use swarm_application::RunnerService;
use tower_http::services::{ServeDir, ServeFile};

pub use api::api;

/// The panel's HTTP server: the API under `/api` and, given the directory
/// the web panel was built into, the panel at every other path.
///
/// A path that is no file of the panel's is answered its `index.html`, so
/// a route of the panel's own, loaded or reloaded in a browser, loads the
/// panel. Paths under `/api` are always the API's, answered in JSON even
/// when they name no endpoint.
pub fn router(service: Arc<RunnerService>, web: Option<&Path>) -> Router {
    let router = Router::new().nest("/api", api(service));
    match web {
        Some(directory) => router.fallback_service(
            ServeDir::new(directory).fallback(ServeFile::new(directory.join("index.html"))),
        ),
        None => router,
    }
}
