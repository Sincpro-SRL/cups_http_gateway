use axum::{Extension, Json};

use crate::entrypoints::http::app_state::AppState;
use crate::entrypoints::http::models::StatusResponse;

pub async fn get_status(Extension(state): Extension<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        cups: state.cups_addr,
    })
}
