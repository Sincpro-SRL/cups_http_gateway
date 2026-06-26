use crate::adapters::cups::client::CupsError;
use crate::adapters::cups::CupsClient;
use crate::domain::job::JobDetail;
use crate::domain::print_options::{DocumentFormat, PrintJobOptions};
use crate::domain::printer::PrinterInfo;

/// Stateless service layer: validates input, delegates to the CUPS adapter.
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

    /// Submit a print job.
    ///
    /// `data` is raw bytes. Callers are responsible for encoding/decoding
    /// (e.g. base64 decode at the HTTP layer before calling this).
    pub async fn submit_job(
        &self,
        printer: &str,
        data: Vec<u8>,
        format: DocumentFormat,
        job_name: Option<&str>,
        options: PrintJobOptions,
    ) -> Result<i32, CupsError> {
        self.cups
            .print_job(printer, data, &format, job_name, &options)
            .await
    }

    pub async fn list_jobs(&self, printer: &str) -> Result<Vec<JobDetail>, CupsError> {
        self.cups.get_jobs(printer).await
    }
}
