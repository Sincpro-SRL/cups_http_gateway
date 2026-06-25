use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};

use crate::entrypoints::http::app_state::AppState;
use crate::entrypoints::http::errors::into_http_error;
use crate::entrypoints::http::models::{
    ErrorResponse, PrintRequest, PrintResponse, PrinterInfoResponse, PrintersResponse,
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
    state
        .service
        .submit_job(&name, &body.content, &body.format, body.job_name.as_deref())
        .await
        .map(|job_id| Json(PrintResponse { ok: true, job_id }))
        .map_err(|e| into_http_error(&e))
}
