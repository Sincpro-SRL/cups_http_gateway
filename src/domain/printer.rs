#[derive(Debug)]
pub struct PrinterInfo {
    pub name: String,
    pub state: String,
    pub queued_jobs: u32,
}

#[derive(Debug, Clone)]
pub struct PrinterCapabilities {
    pub make_and_model: String,
    pub state: String,
    pub state_reasons: Vec<String>,
    /// MIME types this printer accepts (e.g. "application/pdf", "image/jpeg").
    pub formats_supported: Vec<String>,
    /// CUPS media keywords this printer accepts (e.g. `iso_a4`, `custom_80x297mm`).
    pub media_supported: Vec<String>,
    /// Currently configured default media keyword.
    pub media_default: String,
    /// Media physically loaded/ready right now.
    pub media_ready: Vec<String>,
    pub sides_supported: Vec<String>,
    pub sides_default: String,
    pub color_modes_supported: Vec<String>,
    pub color_mode_default: String,
}
