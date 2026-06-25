use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};

use crate::entrypoints::http::app_state::AppState;
use crate::entrypoints::http::errors::into_http_error;
use crate::entrypoints::http::models::{ErrorResponse, JobInfo, JobsResponse};

pub async fn list_jobs(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Result<Json<JobsResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .list_jobs(&name)
        .await
        .map(|jobs| {
            Json(JobsResponse {
                jobs: jobs
                    .into_iter()
                    .map(|j| JobInfo {
                        id: j.id,
                        name: j.name,
                        status: j.state,
                    })
                    .collect(),
            })
        })
        .map_err(|e| into_http_error(&e))
}
