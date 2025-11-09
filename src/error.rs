use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    pub fn validation(details: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorResponse {
                error: "validation_error",
                details: details.into(),
                correlation_id: None,
            },
        }
    }

    pub fn not_found(details: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: ErrorResponse {
                error: "not_found",
                details: details.into(),
                correlation_id: None,
            },
        }
    }

    pub fn driver(details: impl Into<String>) -> Self {
        let correlation_id = Uuid::new_v4().to_string();
        Self {
            status: StatusCode::BAD_GATEWAY,
            body: ErrorResponse {
                error: "driver_error",
                details: details.into(),
                correlation_id: Some(correlation_id),
            },
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_has_expected_shape() {
        let error = ApiError::validation("oops");
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error.body.error, "validation_error");
        assert!(error.body.correlation_id.is_none());
    }

    #[test]
    fn driver_error_provides_correlation_id() {
        let error = ApiError::driver("mongo");
        assert_eq!(error.status(), StatusCode::BAD_GATEWAY);
        assert!(error.body.correlation_id.is_some());
    }

    #[test]
    fn not_found_error_has_expected_shape() {
        let error = ApiError::not_found("document not found");
        assert_eq!(error.status(), StatusCode::NOT_FOUND);
        assert_eq!(error.body.error, "not_found");
        assert_eq!(error.body.details, "document not found");
        assert!(error.body.correlation_id.is_none());
    }

    #[test]
    fn error_serializes_to_json() {
        let error = ApiError::validation("test error");
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validation_error_details_are_preserved() {
        let details = "database must be provided";
        let error = ApiError::validation(details);
        assert_eq!(error.body.details, details);
    }
}
