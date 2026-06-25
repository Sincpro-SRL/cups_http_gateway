use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── HTTP Request bodies ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    /// Plain text or base64-encoded bytes depending on `format`.
    pub content: String,
    /// MIME type: text/plain | application/pdf | application/octet-stream
    pub format: String,
    pub job_name: Option<String>,
    #[allow(dead_code)]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

// ── HTTP Response bodies ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub ok: bool,
    pub version: &'static str,
    pub cups: String,
}

#[derive(Debug, Serialize)]
pub struct PrintersResponse {
    pub printers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PrinterInfoResponse {
    pub name: String,
    pub status: String,
    pub jobs: u32,
}

#[derive(Debug, Serialize)]
pub struct PrintResponse {
    pub ok: bool,
    pub job_id: i32,
}

#[derive(Debug, Serialize)]
pub struct JobInfo {
    pub id: i32,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct JobsResponse {
    pub jobs: Vec<JobInfo>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub error: String,
    pub code: u16,
}
