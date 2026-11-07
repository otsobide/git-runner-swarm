//! The web panel's files, served beside the API from the directory the
//! panel was built into.

use std::fs;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use swarm_application::{RunnerImages, RunnerService};
use swarm_memory::InMemoryRuntime;
use tempfile::TempDir;
use tower::ServiceExt;

const INDEX: &str = "<!doctype html><title>git-runner-swarm</title>";

/// A panel serving a built web panel: an index and one asset.
fn panel() -> (Router, TempDir) {
    let web = tempfile::tempdir().unwrap();
    fs::write(web.path().join("index.html"), INDEX).unwrap();
    fs::create_dir(web.path().join("assets")).unwrap();
    fs::write(web.path().join("assets/app.js"), "console.log('panel')").unwrap();

    let service = Arc::new(RunnerService::new(
        Arc::new(InMemoryRuntime::new()),
        RunnerImages::default(),
    ));
    (swarm_http::router(service, Some(web.path())), web)
}

async fn get(router: &Router, uri: &str) -> (StatusCode, Option<String>, String) {
    let request = Request::get(uri).body(Body::empty()).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_owned());
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        content_type,
        String::from_utf8(body.to_vec()).unwrap(),
    )
}

#[tokio::test]
async fn serves_the_panel_at_the_root() {
    let (router, _web) = panel();

    let (status, content_type, body) = get(&router, "/").await;

    assert_eq!(status, StatusCode::OK);
    assert!(content_type.unwrap().starts_with("text/html"));
    assert_eq!(body, INDEX);
}

#[tokio::test]
async fn serves_the_panels_assets() {
    let (router, _web) = panel();

    let (status, content_type, body) = get(&router, "/assets/app.js").await;

    assert_eq!(status, StatusCode::OK);
    assert!(content_type.unwrap().contains("javascript"));
    assert_eq!(body, "console.log('panel')");
}

#[tokio::test]
async fn serves_the_panel_for_a_route_of_its_own() {
    let (router, _web) = panel();

    let (status, _, body) = get(&router, "/runners/67e55044-10b1-426f-9247-bb680e5fe0c8").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, INDEX);
}

#[tokio::test]
async fn leaves_the_api_to_the_api() {
    let (router, _web) = panel();

    let (health, _, _) = get(&router, "/api/health").await;
    let (status, content_type, body) = get(&router, "/api/nothing-here").await;

    assert_eq!(health, StatusCode::OK);
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert!(body.contains("there is no such endpoint"));
}
