use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::adapters::cups::client::CupsError;
use crate::domain::print_options::{
    ColorMode, DocumentFormat, MediaSize, Orientation, PrintJobOptions, Sides,
};
use crate::entrypoints::http::app_state::AppState;
use crate::entrypoints::http::errors::into_http_error;
use crate::entrypoints::http::models::{
    ErrorResponse, HttpPrintOptions, PrintRequest, PrintResponse, PrinterCapabilitiesResponse,
    PrinterDefaultsResponse, PrinterInfoResponse, PrintersResponse,
};

pub async fn list_printers(
    Extension(state): Extension<AppState>,
) -> Result<Json<PrintersResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .list_printers()
        .await
        .map(|printers| Json(PrintersResponse { printers }))
        .map_err(|e| into_http_error(&e))
}

pub async fn get_printer_defaults(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PrinterDefaultsResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .printer_capabilities(&name)
        .await
        .map(|c| {
            Json(PrinterDefaultsResponse {
                media: c.media_default,
                sides: c.sides_default,
                color_mode: c.color_mode_default,
            })
        })
        .map_err(|e| into_http_error(&e))
}

pub async fn get_printer_capabilities(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PrinterCapabilitiesResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .printer_capabilities(&name)
        .await
        .map(|c| {
            Json(PrinterCapabilitiesResponse {
                make_and_model: c.make_and_model,
                state: c.state,
                state_reasons: c.state_reasons,
                formats_supported: c.formats_supported,
                media_supported: c.media_supported,
                media_default: c.media_default,
                media_ready: c.media_ready,
                sides_supported: c.sides_supported,
                sides_default: c.sides_default,
                color_modes_supported: c.color_modes_supported,
                color_mode_default: c.color_mode_default,
            })
        })
        .map_err(|e| into_http_error(&e))
}

pub async fn get_printer(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PrinterInfoResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .printer_info(&name)
        .await
        .map(|info| {
            Json(PrinterInfoResponse {
                name: info.name,
                status: info.state,
                jobs: info.queued_jobs,
            })
        })
        .map_err(|e| into_http_error(&e))
}

pub async fn print_job(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
    Json(body): Json<PrintRequest>,
) -> Result<Json<PrintResponse>, (StatusCode, Json<ErrorResponse>)> {
    let format = DocumentFormat::from_mime(&body.format);
    let data = decode_content(&body.content, &format).map_err(|e| into_http_error(&e))?;
    let options = map_options(&body.options.unwrap_or_default());

    state
        .service
        .submit_job(&name, data, format, body.job_name.as_deref(), options)
        .await
        .map(|job_id| Json(PrintResponse { ok: true, job_id }))
        .map_err(|e| into_http_error(&e))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Plain text arrives as UTF-8; all other formats must be base64-encoded.
fn decode_content(content: &str, format: &DocumentFormat) -> Result<Vec<u8>, CupsError> {
    if format.is_text() {
        Ok(content.as_bytes().to_vec())
    } else {
        BASE64.decode(content).map_err(CupsError::Base64)
    }
}

fn map_options(http: &HttpPrintOptions) -> PrintJobOptions {
    PrintJobOptions {
        copies: http.copies,
        media: http.media.as_deref().map(MediaSize::from_keyword),
        sides: http.sides.as_deref().and_then(Sides::from_keyword),
        color_mode: http.color_mode.as_deref().and_then(ColorMode::from_keyword),
        orientation: http
            .orientation
            .as_deref()
            .and_then(Orientation::from_keyword),
    }
}
