use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use swarm_application::RunnerService;
use swarm_domain::RunnerId;

use crate::dto::{
    CreateRunnerRequest, ImageResponse, ListQuery, LogsQuery, LogsResponse, RunnerResponse,
};
use crate::error::ApiError;

type Service = State<Arc<RunnerService>>;

/// The API, its endpoints relative to where it is mounted:
///
/// - `GET /health`: whether the panel is up.
/// - `GET /images`: the images runners can be created from.
/// - `GET /runners`: the runners, filtered by the query parameters
///   `forge`, `owner`, `repository`, `scope`, `status`, `label` and `q`.
/// - `POST /runners`: creates a runner and starts it.
/// - `GET /runners/{id}`, `DELETE /runners/{id}`: a runner, and deleting
///   it.
/// - `POST /runners/{id}/start`, `POST /runners/{id}/stop`.
/// - `GET /runners/{id}/logs`: the latest lines it logged, as many as the
///   query parameter `lines` says, 200 unless it is given.
pub fn api(service: Arc<RunnerService>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/images", get(images))
        .route("/runners", get(list).post(create))
        .route("/runners/{id}", get(show).delete(delete))
        .route("/runners/{id}/start", post(start))
        .route("/runners/{id}/stop", post(stop))
        .route("/runners/{id}/logs", get(logs))
        .fallback(no_such_endpoint)
        .with_state(service)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn images(State(service): Service) -> Json<Vec<ImageResponse>> {
    Json(service.images().iter().map(ImageResponse::from).collect())
}

async fn list(
    State(service): Service,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<RunnerResponse>>, ApiError> {
    let Query(query) = query.map_err(|rejection| ApiError::Malformed(rejection.body_text()))?;
    let runners = service.list(&query.into_filter()?).await?;
    Ok(Json(runners.iter().map(RunnerResponse::from).collect()))
}

async fn create(
    State(service): Service,
    body: Result<Json<CreateRunnerRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(request) = body.map_err(|rejection| ApiError::Malformed(rejection.body_text()))?;
    let runner = service.create(request.into_command()?).await?;
    let location = format!("/api/runners/{}", runner.id());
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(RunnerResponse::from(&runner)),
    ))
}

async fn show(
    State(service): Service,
    Path(id): Path<String>,
) -> Result<Json<RunnerResponse>, ApiError> {
    let runner = service.get(runner_id(&id)?).await?;
    Ok(Json(RunnerResponse::from(&runner)))
}

async fn start(
    State(service): Service,
    Path(id): Path<String>,
) -> Result<Json<RunnerResponse>, ApiError> {
    let runner = service.start(runner_id(&id)?).await?;
    Ok(Json(RunnerResponse::from(&runner)))
}

async fn stop(
    State(service): Service,
    Path(id): Path<String>,
) -> Result<Json<RunnerResponse>, ApiError> {
    let runner = service.stop(runner_id(&id)?).await?;
    Ok(Json(RunnerResponse::from(&runner)))
}

async fn delete(State(service): Service, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    service.delete(runner_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn logs(
    State(service): Service,
    Path(id): Path<String>,
    query: Result<Query<LogsQuery>, QueryRejection>,
) -> Result<Json<LogsResponse>, ApiError> {
    let Query(query) = query.map_err(|rejection| ApiError::Malformed(rejection.body_text()))?;
    let lines = service.logs(runner_id(&id)?, query.into_tail()?).await?;
    Ok(Json(LogsResponse::from(lines.as_slice())))
}

async fn no_such_endpoint() -> ApiError {
    ApiError::NoSuchEndpoint
}

/// The id a path names. One that is not even shaped like an id names no
/// runner, which is what is answered.
fn runner_id(id: &str) -> Result<RunnerId, ApiError> {
    id.parse()
        .map_err(|_| ApiError::NoSuchRunner(id.to_owned()))
}
