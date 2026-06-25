use axum::{http::StatusCode, Json};

use crate::adapters::cups::client::CupsError;

use super::models::ErrorResponse;

/// Maps a [`CupsError`] to an axum-compatible (`StatusCode`, JSON body) pair.
pub fn into_http_error(e: &CupsError) -> (StatusCode, Json<ErrorResponse>) {
    let (http_status, message) = match e {
        CupsError::PrinterNotFound(name) => {
            (StatusCode::NOT_FOUND, format!("printer not found: {name}"))
        }
        CupsError::InvalidUri(_) | CupsError::MissingAttribute(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        CupsError::IppStatus(_) | CupsError::Transport(_) | CupsError::Base64(_) => {
            (StatusCode::BAD_GATEWAY, e.to_string())
        }
    };

    let body = ErrorResponse {
        ok: false,
        error: message,
        code: http_status.as_u16(),
    };

    (http_status, Json(body))
}
