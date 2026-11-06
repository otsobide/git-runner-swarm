use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use swarm_application::RunnerError;
use swarm_application::ports::RuntimeError;
use swarm_domain::InvalidValue;

/// Why a request failed, answered as a status and a JSON body:
/// `{"error": "why", "field": "which"}`, the field there when one value is
/// to blame, so a form can point at it.
#[derive(Debug)]
pub(crate) enum ApiError {
    /// A value the domain refused.
    Invalid(InvalidValue),
    /// A body or query that is not what the endpoint takes.
    Malformed(String),
    /// A path naming no runner, not even by the shape of its id.
    NoSuchRunner(String),
    /// A path naming no endpoint.
    NoSuchEndpoint,
    /// A use case that could not be carried out.
    Runner(RunnerError),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'static str>,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Malformed(_) => StatusCode::BAD_REQUEST,
            ApiError::NoSuchRunner(_) | ApiError::NoSuchEndpoint => StatusCode::NOT_FOUND,
            ApiError::Runner(error) => match error {
                RunnerError::NotFound(_) | RunnerError::Runtime(RuntimeError::NotFound(_)) => {
                    StatusCode::NOT_FOUND
                }
                RunnerError::NameTaken { .. } => StatusCode::CONFLICT,
                RunnerError::UnknownImage(_)
                | RunnerError::NoImageForForge(_)
                | RunnerError::ImageForgeMismatch { .. } => StatusCode::UNPROCESSABLE_ENTITY,
                RunnerError::Runtime(RuntimeError::Unreachable(_)) => {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                RunnerError::Runtime(
                    RuntimeError::ImageUnavailable { .. } | RuntimeError::Failed(_),
                ) => StatusCode::BAD_GATEWAY,
            },
        }
    }

    /// The value to blame, named as the domain names it.
    fn field(&self) -> Option<&'static str> {
        match self {
            ApiError::Invalid(error) => Some(error.field()),
            ApiError::Runner(RunnerError::NameTaken { .. }) => Some("runner name"),
            ApiError::Runner(
                RunnerError::UnknownImage(_) | RunnerError::ImageForgeMismatch { .. },
            ) => Some("image"),
            ApiError::Runner(RunnerError::NoImageForForge(_)) => Some("forge"),
            _ => None,
        }
    }

    fn message(&self) -> String {
        match self {
            ApiError::Invalid(error) => error.to_string(),
            ApiError::Malformed(reason) => reason.clone(),
            ApiError::NoSuchRunner(id) => format!("there is no runner {id}"),
            ApiError::NoSuchEndpoint => "there is no such endpoint".to_owned(),
            ApiError::Runner(error) => error.to_string(),
        }
    }
}

impl From<InvalidValue> for ApiError {
    fn from(error: InvalidValue) -> Self {
        ApiError::Invalid(error)
    }
}

impl From<RunnerError> for ApiError {
    fn from(error: RunnerError) -> Self {
        ApiError::Runner(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = ErrorBody {
            error: self.message(),
            field: self.field(),
        };
        if status.is_server_error() {
            tracing::warn!(status = status.as_u16(), "{}", body.error);
        }
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use swarm_domain::{Forge, ImageReference, RunnerId, RunnerName, RunnerScope};

    use super::*;

    fn status(error: impl Into<ApiError>) -> StatusCode {
        error.into().status()
    }

    async fn body(error: ApiError) -> Value {
        let body = axum::body::to_bytes(error.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn name_taken() -> RunnerError {
        RunnerError::NameTaken {
            name: RunnerName::new("builder").unwrap(),
            scope: RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
        }
    }

    #[test]
    fn answers_a_refused_value_as_unprocessable() {
        let refused = RunnerName::new("two words").unwrap_err();

        assert_eq!(status(refused), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn answers_a_runner_that_is_not_there_as_not_found() {
        let id = RunnerId::generate();

        assert_eq!(status(RunnerError::NotFound(id)), StatusCode::NOT_FOUND);
        assert_eq!(
            status(ApiError::NoSuchRunner("x".to_owned())),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn answers_a_name_already_taken_as_a_conflict() {
        assert_eq!(status(name_taken()), StatusCode::CONFLICT);
    }

    #[test]
    fn answers_an_image_it_cannot_create_from_as_unprocessable() {
        assert_eq!(
            status(RunnerError::UnknownImage("gitea".to_owned())),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            status(RunnerError::NoImageForForge(Forge::GitHub)),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn answers_an_engine_out_of_reach_as_unavailable_and_a_failing_one_as_a_bad_gateway() {
        let unreachable = RunnerError::Runtime(RuntimeError::Unreachable("no socket".to_owned()));
        let failed = RunnerError::Runtime(RuntimeError::Failed("conflict".to_owned()));
        let unpullable = RunnerError::Runtime(RuntimeError::ImageUnavailable {
            image: ImageReference::new("nope:never").unwrap(),
            reason: "manifest unknown".to_owned(),
        });

        assert_eq!(status(unreachable), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(status(failed), StatusCode::BAD_GATEWAY);
        assert_eq!(status(unpullable), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn says_why_and_which_field_is_to_blame() {
        let refused = RunnerName::new("two words").unwrap_err();

        assert_eq!(
            body(refused.into()).await,
            json!({
                "error": "invalid runner name: ` ` is not allowed; only letters, digits, `-`, `_` and `.` are",
                "field": "runner name",
            })
        );
        assert_eq!(body(name_taken().into()).await["field"], "runner name");
    }

    #[tokio::test]
    async fn names_no_field_when_no_one_value_is_to_blame() {
        let body = body(ApiError::NoSuchEndpoint).await;

        assert_eq!(body, json!({ "error": "there is no such endpoint" }));
    }
}
