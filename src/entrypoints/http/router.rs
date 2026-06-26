use axum::{routing::get, routing::post, Extension, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::app_state::AppState;
use super::routes::{jobs, printers, status};

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/status", get(status::get_status))
        .route("/printers", get(printers::list_printers))
        .route("/printers/:name", get(printers::get_printer))
        .route(
            "/printers/:name/capabilities",
            get(printers::get_printer_capabilities),
        )
        .route(
            "/printers/:name/defaults",
            get(printers::get_printer_defaults),
        )
        .route("/printers/:name/print", post(printers::print_job))
        .route("/printers/:name/jobs", get(jobs::list_jobs))
        .layer(Extension(state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
