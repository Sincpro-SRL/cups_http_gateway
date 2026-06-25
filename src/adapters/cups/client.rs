use ipp::operation::{IppOperation, PrintJob};
use ipp::prelude::*;
use thiserror::Error;

use crate::domain::job::JobDetail;
use crate::domain::printer::PrinterInfo;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum CupsError {
    #[error("IPP transport error: {0}")]
    Transport(#[from] IppError),

    #[error("IPP status error: {0}")]
    IppStatus(String),

    #[error("Printer not found: {0}")]
    PrinterNotFound(String),

    #[error("Missing attribute: {0}")]
    MissingAttribute(&'static str),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CupsClient {
    host: String,
    port: u16,
}

impl CupsClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    // The ipp crate sends IPP over plain HTTP — URI scheme must be http://
    fn printer_uri(&self, printer: &str) -> Result<Uri, CupsError> {
        format!("http://{}:{}/printers/{}", self.host, self.port, printer)
            .parse()
            .map_err(|e| CupsError::InvalidUri(format!("{e}")))
    }

    fn root_uri(&self) -> Result<Uri, CupsError> {
        format!("http://{}:{}/", self.host, self.port)
            .parse()
            .map_err(|e| CupsError::InvalidUri(format!("{e}")))
    }

    // ── Public interface ──────────────────────────────────────────────────────

    pub async fn get_printers(&self) -> Result<Vec<String>, CupsError> {
        let uri = self.root_uri()?;
        let op = IppOperationBuilder::cups().get_printers();
        let client = AsyncIppClient::new(uri);
        let resp = client.send(op).await?;
        assert_success(&resp)?;

        let mut names = Vec::new();
        for group in resp.attributes().groups_of(DelimiterTag::PrinterAttributes) {
            if let Some(attr) = group.attributes().get("printer-name") {
                if let IppValue::NameWithoutLanguage(name) = attr.value() {
                    names.push(name.clone());
                }
            }
        }
        Ok(names)
    }

    pub async fn get_printer_info(&self, printer: &str) -> Result<PrinterInfo, CupsError> {
        let uri = self.printer_uri(printer)?;
        let op = IppOperationBuilder::get_printer_attributes(uri.clone())
            .attributes(["printer-name", "printer-state", "queued-job-count"])
            .build();
        let client = AsyncIppClient::new(uri);
        let resp = client.send(op).await?;

        if resp.header().status_code() == StatusCode::ClientErrorNotFound {
            return Err(CupsError::PrinterNotFound(printer.to_owned()));
        }
        assert_success(&resp)?;

        let mut info = PrinterInfo {
            name: printer.to_owned(),
            state: "unknown".to_owned(),
            queued_jobs: 0,
        };

        for group in resp.attributes().groups_of(DelimiterTag::PrinterAttributes) {
            let attrs = group.attributes();
            if let Some(attr) = attrs.get("printer-state") {
                info.state = printer_state_str(attr.value());
            }
            if let Some(attr) = attrs.get("queued-job-count") {
                if let IppValue::Integer(n) = attr.value() {
                    info.queued_jobs = u32::try_from(*n).unwrap_or(0);
                }
            }
        }

        Ok(info)
    }

    pub async fn print_job(
        &self,
        printer: &str,
        data: Vec<u8>,
        mime_type: &str,
        job_name: Option<&str>,
    ) -> Result<i32, CupsError> {
        let uri = self.printer_uri(printer)?;
        let payload = IppPayload::new(std::io::Cursor::new(data));

        // Build the PrintJob and convert to a raw request so we can inject
        // document-format into the correct group (OperationAttributes).
        let op = PrintJob::new(uri.clone(), payload, None::<&str>, job_name);
        let mut req = op.into_ipp_request();
        req.attributes_mut().add(
            DelimiterTag::OperationAttributes,
            IppAttribute::new(
                "document-format",
                IppValue::MimeMediaType(mime_type.to_owned()),
            ),
        );

        let client = AsyncIppClient::new(uri);
        let resp = client.send(req).await?;

        if resp.header().status_code() == StatusCode::ClientErrorNotFound {
            return Err(CupsError::PrinterNotFound(printer.to_owned()));
        }
        assert_success(&resp)?;

        for group in resp.attributes().groups_of(DelimiterTag::JobAttributes) {
            if let Some(attr) = group.attributes().get("job-id") {
                if let IppValue::Integer(id) = attr.value() {
                    return Ok(*id);
                }
            }
        }

        Err(CupsError::MissingAttribute("job-id"))
    }

    pub async fn get_jobs(&self, printer: &str) -> Result<Vec<JobDetail>, CupsError> {
        let uri = self.printer_uri(printer)?;

        // GetJobs has no high-level builder in ipp v4 — build the request directly.
        let mut req =
            IppRequestResponse::new(IppVersion::v1_1(), Operation::GetJobs, Some(uri.clone()));
        req.attributes_mut().add(
            DelimiterTag::OperationAttributes,
            IppAttribute::new(
                IppAttribute::REQUESTED_ATTRIBUTES,
                IppValue::Array(vec![
                    IppValue::Keyword("job-id".to_owned()),
                    IppValue::Keyword("job-name".to_owned()),
                    IppValue::Keyword("job-state".to_owned()),
                ]),
            ),
        );
        req.attributes_mut().add(
            DelimiterTag::OperationAttributes,
            IppAttribute::new("which-jobs", IppValue::Keyword("all".to_owned())),
        );

        let client = AsyncIppClient::new(uri);
        let resp = client.send(req).await?;

        if resp.header().status_code() == StatusCode::ClientErrorNotFound {
            return Err(CupsError::PrinterNotFound(printer.to_owned()));
        }
        assert_success(&resp)?;

        let mut jobs = Vec::new();
        for group in resp.attributes().groups_of(DelimiterTag::JobAttributes) {
            let attrs = group.attributes();

            let id = attrs
                .get("job-id")
                .and_then(|a| {
                    if let IppValue::Integer(n) = a.value() {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let name = attrs
                .get("job-name")
                .map(|a| match a.value() {
                    IppValue::NameWithoutLanguage(s) | IppValue::TextWithoutLanguage(s) => {
                        s.clone()
                    }
                    _ => String::new(),
                })
                .unwrap_or_default();

            let state = attrs
                .get("job-state")
                .map_or_else(|| "unknown".to_owned(), |a| job_state_str(a.value()));

            jobs.push(JobDetail { id, name, state });
        }

        Ok(jobs)
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn assert_success(resp: &IppRequestResponse) -> Result<(), CupsError> {
    let code = resp.header().status_code();
    if code.is_success() {
        Ok(())
    } else {
        Err(CupsError::IppStatus(format!("{code:?}")))
    }
}

// ── Value helpers ─────────────────────────────────────────────────────────────

fn printer_state_str(value: &IppValue) -> String {
    match value {
        IppValue::Enum(3) => "idle".to_owned(),
        IppValue::Enum(4) => "processing".to_owned(),
        IppValue::Enum(5) => "stopped".to_owned(),
        IppValue::Enum(n) => format!("unknown({n})"),
        _ => "unknown".to_owned(),
    }
}

fn job_state_str(value: &IppValue) -> String {
    match value {
        IppValue::Enum(3) => "pending".to_owned(),
        IppValue::Enum(4) => "pending-held".to_owned(),
        IppValue::Enum(5) => "processing".to_owned(),
        IppValue::Enum(6) => "processing-stopped".to_owned(),
        IppValue::Enum(7) => "canceled".to_owned(),
        IppValue::Enum(8) => "aborted".to_owned(),
        IppValue::Enum(9) => "completed".to_owned(),
        IppValue::Enum(n) => format!("unknown({n})"),
        _ => "unknown".to_owned(),
    }
}
