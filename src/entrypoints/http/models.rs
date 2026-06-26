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
    /// Append an ESC/POS cut command after the payload. `"full"` (default) or `"partial"`.
    /// Only applies to raw ESC/POS streams (`application/octet-stream`).
    pub cut: Option<String>,
    /// When `true`, query the printer capabilities first and automatically fall back
    /// to printer defaults for any unsupported option (media, sides, color mode).
    /// An unsupported format is always an error regardless of this flag.
    #[serde(default)]
    pub smart: bool,
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
pub struct PrinterDefaultsResponse {
    pub media: String,
    pub sides: String,
    pub color_mode: String,
}

#[derive(Debug, Serialize)]
pub struct PrinterCapabilitiesResponse {
    pub make_and_model: String,
    pub state: String,
    pub state_reasons: Vec<String>,
    pub formats_supported: Vec<String>,
    pub media_supported: Vec<String>,
    pub media_default: String,
    pub media_ready: Vec<String>,
    pub sides_supported: Vec<String>,
    pub sides_default: String,
    pub color_modes_supported: Vec<String>,
    pub color_mode_default: String,
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
