//! The API end to end: requests through the router, the application
//! behind it with the official GitHub image's adapter, and the in-memory
//! runtime standing in for a container engine.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use serde_json::{Value, json};
use swarm_application::ports::RunnerImage;
use swarm_application::{RunnerImages, RunnerService};
use swarm_domain::ImageReference;
use swarm_github::GitHubOfficialImage;
use swarm_memory::InMemoryRuntime;
use tower::ServiceExt;

const TOKEN: &str = "AESOHC2SEXAMPLETOKEN";

struct Panel {
    router: Router,
    runtime: Arc<InMemoryRuntime>,
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

fn panel() -> Panel {
    let runtime = Arc::new(InMemoryRuntime::new());
    let images =
        RunnerImages::new([Arc::new(GitHubOfficialImage::default()) as Arc<dyn RunnerImage>]);
    let service = Arc::new(RunnerService::new(runtime.clone(), images));
    Panel {
        router: swarm_http::router(service, None),
        runtime,
    }
}

impl Panel {
    async fn send(&self, method: Method, uri: &str, body: Option<Value>) -> Answer {
        let request = Request::builder().method(method).uri(uri);
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        Answer {
            status,
            headers,
            body,
        }
    }

    async fn get(&self, uri: &str) -> Answer {
        self.send(Method::GET, uri, None).await
    }

    async fn post(&self, uri: &str, body: Option<Value>) -> Answer {
        self.send(Method::POST, uri, body).await
    }

    async fn delete(&self, uri: &str) -> Answer {
        self.send(Method::DELETE, uri, None).await
    }

    /// Creates a runner, which must succeed, and answers it.
    async fn created(&self, url: &str, name: &str) -> Value {
        let answer = self.post("/api/runners", Some(runner(url, name))).await;
        assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
        answer.body
    }
}

fn runner(url: &str, name: &str) -> Value {
    json!({ "url": url, "name": name, "token": TOKEN, "labels": ["gpu"] })
}

fn names(runners: &Value) -> Vec<&str> {
    let mut names: Vec<&str> = runners
        .as_array()
        .unwrap()
        .iter()
        .map(|runner| runner["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    names
}

#[tokio::test]
async fn says_it_is_up() {
    let answer = panel().get("/api/health").await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn lists_the_images_runners_can_be_created_from() {
    let answer = panel().get("/api/images").await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body[0]["key"], "github-official");
    assert_eq!(answer.body[0]["forge"], "github");
    assert_eq!(
        answer.body[0]["image"],
        "ghcr.io/actions/actions-runner:latest"
    );
}

#[tokio::test]
async fn creates_a_runner_running_after_pulling_its_image() {
    let panel = panel();

    let answer = panel
        .post(
            "/api/runners",
            Some(runner("https://github.com/otsobide/mindweaver", "builder")),
        )
        .await;

    assert_eq!(answer.status, StatusCode::CREATED);
    let id = answer.body["id"].as_str().unwrap();
    assert_eq!(
        answer.headers[header::LOCATION],
        format!("/api/runners/{id}")
    );
    assert_eq!(answer.body["name"], "builder");
    assert_eq!(answer.body["owner"], "otsobide");
    assert_eq!(answer.body["repository"], "mindweaver");
    assert_eq!(answer.body["labels"], json!(["gpu"]));
    assert_eq!(answer.body["lifecycle"], "persistent");
    assert_eq!(answer.body["status"], "running");
    assert!(
        panel
            .runtime
            .has_image(&ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap())
    );
}

#[tokio::test]
async fn creates_a_runner_limited_as_it_is_asked_and_answers_its_limits() {
    let panel = panel();
    let mut body = runner("https://github.com/otsobide/mindweaver", "builder");
    body["cpus"] = json!(1.5);
    body["memory"] = json!("2048m");

    let answer = panel.post("/api/runners", Some(body)).await;
    let id = answer.body["id"].as_str().unwrap().to_owned();
    let shown = panel.get(&format!("/api/runners/{id}")).await;

    assert_eq!(answer.status, StatusCode::CREATED);
    assert_eq!(answer.body["cpus"], "1.5");
    assert_eq!(answer.body["memory"], "2g");
    assert_eq!(shown.body["cpus"], "1.5");
    assert_eq!(shown.body["memory"], "2g");
}

#[tokio::test]
async fn gives_a_runner_the_hosts_docker_only_when_asked() {
    let panel = panel();
    let mut asking = runner("https://github.com/otsobide/mindweaver", "given");
    asking["host_docker"] = json!(true);

    let given = panel.post("/api/runners", Some(asking)).await;
    let isolated = panel
        .created("https://github.com/otsobide/mindweaver", "isolated")
        .await;

    assert_eq!(given.status, StatusCode::CREATED);
    assert_eq!(given.body["host_docker"], true);
    assert_eq!(isolated["host_docker"], false);
}

#[tokio::test]
async fn refuses_a_limit_naming_it() {
    for (field, value, refused) in [
        ("cpus", json!("0"), "CPU limit"),
        ("memory", json!("512"), "memory limit"),
    ] {
        let mut body = runner("https://github.com/otsobide/mindweaver", "builder");
        body[field] = value;

        let answer = panel().post("/api/runners", Some(body)).await;

        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
        assert_eq!(answer.body["field"], refused, "{field}");
    }
}

#[tokio::test]
async fn never_answers_the_token_a_runner_was_created_with() {
    let panel = panel();
    let created = panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;

    let listed = panel.get("/api/runners").await;

    assert!(!created.to_string().contains(TOKEN));
    assert!(!listed.body.to_string().contains(TOKEN));
}

#[tokio::test]
async fn refuses_a_runner_naming_the_field_to_blame() {
    let mut body = runner("https://github.com/otsobide/mindweaver", "builder");
    body["url"] = json!("github.com/otsobide/mindweaver");

    let answer = panel().post("/api/runners", Some(body)).await;

    assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(answer.body["field"], "URL");
}

#[tokio::test]
async fn refuses_an_image_it_does_not_have() {
    let mut body = runner("https://github.com/otsobide/mindweaver", "builder");
    body["image"] = json!("gitea-act-runner");

    let answer = panel().post("/api/runners", Some(body)).await;

    assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(answer.body["field"], "image");
}

#[tokio::test]
async fn refuses_a_body_that_is_not_a_runners() {
    let panel = panel();
    let mut unknown_field = runner("https://github.com/otsobide/mindweaver", "builder");
    unknown_field["ephemeral"] = json!(true);

    for body in [json!({ "name": "builder" }), unknown_field, json!([1, 2])] {
        let answer = panel.post("/api/runners", Some(body)).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
        assert!(answer.body["error"].is_string());
    }
}

#[tokio::test]
async fn refuses_a_name_another_runner_goes_by_in_the_scope() {
    let panel = panel();
    panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;

    let answer = panel
        .post(
            "/api/runners",
            Some(runner("https://github.com/otsobide/mindweaver", "Builder")),
        )
        .await;

    assert_eq!(answer.status, StatusCode::CONFLICT);
    assert_eq!(answer.body["field"], "runner name");
}

#[tokio::test]
async fn lists_runners_filtered_by_owner_repository_status_and_text() {
    let panel = panel();
    panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;
    panel.created("https://github.com/otsobide", "shared").await;
    let other = panel
        .created("https://github.com/someone/else", "deployer")
        .await;
    panel
        .post(
            &format!("/api/runners/{}/stop", other["id"].as_str().unwrap()),
            None,
        )
        .await;

    assert_eq!(
        names(&panel.get("/api/runners").await.body),
        ["builder", "deployer", "shared"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?owner=otsobide").await.body),
        ["builder", "shared"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?repository=mindweaver").await.body),
        ["builder"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?scope=owner").await.body),
        ["shared"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?status=exited").await.body),
        ["deployer"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?q=deploy").await.body),
        ["deployer"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?label=GPU&owner=someone").await.body),
        ["deployer"]
    );
    assert_eq!(
        names(&panel.get("/api/runners?owner=&status=").await.body).len(),
        3
    );
}

#[tokio::test]
async fn refuses_a_filter_no_runner_could_meet_or_does_not_know() {
    let panel = panel();

    let invalid = panel.get("/api/runners?status=online").await;
    let unknown = panel.get("/api/runners?colour=blue").await;

    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(invalid.body["field"], "status");
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn answers_a_runner_by_its_id() {
    let panel = panel();
    let created = panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;

    let answer = panel
        .get(&format!("/api/runners/{}", created["id"].as_str().unwrap()))
        .await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body, created);
}

#[tokio::test]
async fn answers_not_found_for_an_id_naming_no_runner() {
    let panel = panel();

    for uri in [
        "/api/runners/67e55044-10b1-426f-9247-bb680e5fe0c8",
        "/api/runners/not-an-id",
    ] {
        let answer = panel.get(uri).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{uri}");
        assert!(
            answer.body["error"]
                .as_str()
                .unwrap()
                .starts_with("there is no runner")
        );
    }
}

#[tokio::test]
async fn stops_and_starts_a_runner() {
    let panel = panel();
    let created = panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;
    let id = created["id"].as_str().unwrap();

    let stopped = panel.post(&format!("/api/runners/{id}/stop"), None).await;
    let started = panel.post(&format!("/api/runners/{id}/start"), None).await;

    assert_eq!(stopped.status, StatusCode::OK);
    assert_eq!(stopped.body["status"], "exited");
    assert_eq!(stopped.body["exit_code"], 0);
    assert_eq!(started.body["status"], "running");
    assert_eq!(started.body["exit_code"], Value::Null);
}

#[tokio::test]
async fn deletes_a_runner() {
    let panel = panel();
    let created = panel
        .created("https://github.com/otsobide/mindweaver", "builder")
        .await;
    let uri = format!("/api/runners/{}", created["id"].as_str().unwrap());

    let deleted = panel.delete(&uri).await;

    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(panel.get(&uri).await.status, StatusCode::NOT_FOUND);
    assert_eq!(panel.delete(&uri).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn answers_an_endpoint_it_does_not_have_in_json() {
    let answer = panel().get("/api/nothing-here").await;

    assert_eq!(answer.status, StatusCode::NOT_FOUND);
    assert_eq!(answer.body, json!({ "error": "there is no such endpoint" }));
}
