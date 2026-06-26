use serde::{Deserialize, Serialize};

// ── HTTP Request bodies ───────────────────────────────────────────────────────

/// Job attributes the caller can set per-request. All fields are optional;
/// omitted fields fall back to the printer's configured CUPS defaults.
#[derive(Debug, Deserialize, Default)]
pub struct HttpPrintOptions {
    /// Number of copies (e.g. 3).
    pub copies: Option<u32>,
    /// CUPS media keyword: `iso_a4`, `na_letter`, `custom_80x297mm`, etc.
    pub media: Option<String>,
    /// `"one-sided"` | `"two-sided-long-edge"` | `"two-sided-short-edge"`
    pub sides: Option<String>,
    /// `"color"` | `"monochrome"` | `"auto"`
    pub color_mode: Option<String>,
    /// `"portrait"` | `"landscape"` | `"reverse-portrait"` | `"reverse-landscape"`
    pub orientation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    /// Plain UTF-8 text for `text/plain`; base64-encoded bytes for all other formats.
    pub content: String,
    /// MIME type: `text/plain`, `application/pdf`, `image/png`, `image/jpeg`,
    /// `application/postscript`, or any custom type (e.g. `application/octet-stream`
    /// for raw ESC/POS or ZPL).
    pub format: String,
    pub job_name: Option<String>,
    pub options: Option<HttpPrintOptions>,
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
