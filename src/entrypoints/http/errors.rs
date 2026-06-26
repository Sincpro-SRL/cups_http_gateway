use axum::{http::StatusCode, Json};

use crate::adapters::cups::client::CupsError;

use super::models::ErrorResponse;

/// Maps a [`CupsError`] to an axum-compatible (`StatusCode`, JSON body) pair.
pub fn into_http_error(e: &CupsError) -> (StatusCode, Json<ErrorResponse>) {
    let (http_status, message) = match e {
        CupsError::PrinterNotFound(name) => {
            (StatusCode::NOT_FOUND, format!("printer not found: {name}"))
        }
        CupsError::Base64(_) => (
            StatusCode::BAD_REQUEST,
            "content is not valid base64".to_owned(),
        ),
        CupsError::FormatNotSupported {
            requested,
            supported,
        } => (
            StatusCode::BAD_REQUEST,
            format!(
                "format '{requested}' not supported by this printer; accepted: {}",
                supported.join(", ")
            ),
        ),
        CupsError::IppStatus(raw) => ipp_status_to_http(raw),
        CupsError::InvalidUri(_) | CupsError::MissingAttribute(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        CupsError::Transport(_) => (StatusCode::BAD_GATEWAY, e.to_string()),
        CupsError::ConversionError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    (
        http_status,
        Json(ErrorResponse {
            ok: false,
            error: message,
            code: http_status.as_u16(),
        }),
    )
}

/// `ClientError` IPP codes → 400; everything else → 502.
fn ipp_status_to_http(raw: &str) -> (StatusCode, String) {
    if raw.contains("DocumentFormatNotSupported") {
        return (
            StatusCode::BAD_REQUEST,
            format!("printer does not support this document format: {raw}"),
        );
    }
    if raw.contains("AttributesNotSupported") || raw.contains("AttributeValuesNotSupported") {
        return (
            StatusCode::BAD_REQUEST,
            format!("printer does not support one or more job options: {raw}"),
        );
    }
    if raw.contains("ClientError") {
        return (
            StatusCode::BAD_REQUEST,
            format!("invalid print request: {raw}"),
        );
    }
    (StatusCode::BAD_GATEWAY, format!("CUPS error: {raw}"))
}
