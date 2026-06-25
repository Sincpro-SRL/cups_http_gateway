use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::adapters::cups::client::CupsError;
use crate::adapters::cups::CupsClient;
use crate::domain::job::JobDetail;
use crate::domain::printer::PrinterInfo;

/// Stateless service layer: validates input, encodes content, delegates to the CUPS adapter.
/// Does not depend on any HTTP primitives — can be used by any transport layer.
pub struct PrinterService {
    cups: CupsClient,
}

impl PrinterService {
    pub fn new(cups: CupsClient) -> Self {
        Self { cups }
    }

    pub async fn list_printers(&self) -> Result<Vec<String>, CupsError> {
        self.cups.get_printers().await
    }

    pub async fn printer_info(&self, name: &str) -> Result<PrinterInfo, CupsError> {
        self.cups.get_printer_info(name).await
    }

    pub async fn submit_job(
        &self,
        printer: &str,
        content: &str,
        format: &str,
        job_name: Option<&str>,
    ) -> Result<i32, CupsError> {
        let data = content_to_bytes(content, format)?;
        self.cups.print_job(printer, data, format, job_name).await
    }

    pub async fn list_jobs(&self, printer: &str) -> Result<Vec<JobDetail>, CupsError> {
        self.cups.get_jobs(printer).await
    }
}

/// For text/plain: use raw UTF-8 bytes.
/// For binary formats (PDF, etc.): expect base64-encoded content.
fn content_to_bytes(content: &str, format: &str) -> Result<Vec<u8>, CupsError> {
    if format == "text/plain" {
        Ok(content.as_bytes().to_vec())
    } else {
        Ok(BASE64.decode(content)?)
    }
}
