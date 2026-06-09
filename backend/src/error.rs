//! FHub Domain Errors
//!
//! Typed error definitions for all application domains.
//! Replaces generic `anyhow` errors with specific, actionable error types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

/// Main application error type
#[derive(Debug)]
pub enum FHubError {
    // Activity errors
    DownloadNotFound(uuid::Uuid),
    DownloadAlreadyExists(String),
    DownloadInvalidState { id: uuid::Uuid, expected: String, actual: String },

    // Activity group errors
    BatchNotFound(String),
    BatchEmpty(String),

    // Database errors
    Database(String),
    DatabaseConnection(String),

    // Source/provider errors
    HostNotFound(String),
    HostAuthFailed(String),
    HostRateLimited { host: String, retry_after: Option<u64> },

    // Validation errors
    InvalidUuid(String),
    InvalidRequest(String),

    // External service errors
    TmdbError(String),
    FshareError(String),
    FHubServiceError { service: String, message: String },

    // Generic
    Internal(String),
}

impl fmt::Display for FHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DownloadNotFound(id) => write!(f, "FHUB activity was not found: {}", id),
            Self::DownloadAlreadyExists(code) => write!(f, "FHUB activity already exists: {}", code),
            Self::DownloadInvalidState { id, expected, actual } => {
                write!(f, "FHUB activity {} is in an invalid state: expected {}, got {}", id, expected, actual)
            }
            Self::BatchNotFound(id) => write!(f, "FHUB activity group was not found: {}", id),
            Self::BatchEmpty(id) => write!(f, "FHUB activity group is empty: {}", id),
            Self::Database(msg) => write!(f, "FHUB database error: {}", msg),
            Self::DatabaseConnection(msg) => write!(f, "FHUB database connection error: {}", msg),
            Self::HostNotFound(host) => write!(f, "FHUB source provider was not found: {}", host),
            Self::HostAuthFailed(host) => write!(f, "FHUB could not verify source provider credentials: {}", host),
            Self::HostRateLimited { host, retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "FHUB source provider {} is rate limited, retry after {}s", host, secs)
                } else {
                    write!(f, "FHUB source provider {} is rate limited", host)
                }
            }
            Self::InvalidUuid(s) => write!(f, "FHUB received an invalid UUID: {}", s),
            Self::InvalidRequest(msg) => write!(f, "FHUB received an invalid request: {}", msg),
            Self::TmdbError(msg) => write!(f, "FHUB metadata service error: {}", msg),
            Self::FshareError(msg) => write!(f, "FHUB source service error: {}", msg),
            Self::FHubServiceError { service, message } => {
                write!(f, "FHUB {} service error: {}", service, message)
            }
            Self::Internal(msg) => write!(f, "FHUB internal error: {}", msg),
        }
    }
}

impl std::error::Error for FHubError {}

/// HTTP error response body
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for FHubError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            // 404 Not Found
            FHubError::DownloadNotFound(_) => (StatusCode::NOT_FOUND, "DOWNLOAD_NOT_FOUND", self.to_string(), None),
            FHubError::BatchNotFound(_) => (StatusCode::NOT_FOUND, "BATCH_NOT_FOUND", self.to_string(), None),
            FHubError::HostNotFound(_) => (StatusCode::NOT_FOUND, "HOST_NOT_FOUND", self.to_string(), None),

            // 400 Bad Request
            FHubError::InvalidUuid(_) => (StatusCode::BAD_REQUEST, "INVALID_UUID", self.to_string(), None),
            FHubError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST", self.to_string(), None),
            FHubError::BatchEmpty(_) => (StatusCode::BAD_REQUEST, "BATCH_EMPTY", self.to_string(), None),

            // 409 Conflict
            FHubError::DownloadAlreadyExists(_) => (StatusCode::CONFLICT, "DOWNLOAD_EXISTS", self.to_string(), None),
            FHubError::DownloadInvalidState { .. } => (StatusCode::CONFLICT, "INVALID_STATE", self.to_string(), None),

            // 401 Unauthorized
            FHubError::HostAuthFailed(_) => (StatusCode::UNAUTHORIZED, "AUTH_FAILED", self.to_string(), None),

            // 429 Too Many Requests
            FHubError::HostRateLimited { retry_after, .. } => {
                let msg = self.to_string();
                let details = retry_after.map(|s| format!("retry_after: {}", s));
                (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", msg, details)
            }

            // 503 Service Unavailable
            FHubError::TmdbError(_) => (StatusCode::SERVICE_UNAVAILABLE, "TMDB_ERROR", self.to_string(), None),
            FHubError::FshareError(_) => (StatusCode::SERVICE_UNAVAILABLE, "FSHARE_ERROR", self.to_string(), None),
            FHubError::FHubServiceError { .. } => (StatusCode::SERVICE_UNAVAILABLE, "FHUB_ERROR", self.to_string(), None),
            FHubError::DatabaseConnection(_) => (StatusCode::SERVICE_UNAVAILABLE, "DB_UNAVAILABLE", self.to_string(), None),

            // 500 Internal Server Error
            FHubError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", self.to_string(), None),
            FHubError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", self.to_string(), None),
        };

        let body = ErrorResponse {
            error: message,
            code: code.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

// Convenience conversions
impl From<rusqlite::Error> for FHubError {
    fn from(err: rusqlite::Error) -> Self {
        FHubError::Database(err.to_string())
    }
}

impl From<uuid::Error> for FHubError {
    fn from(err: uuid::Error) -> Self {
        FHubError::InvalidUuid(err.to_string())
    }
}

/// Result type alias for FHub operations
pub type FHubResult<T> = Result<T, FHubError>;
