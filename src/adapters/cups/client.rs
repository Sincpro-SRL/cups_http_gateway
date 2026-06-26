use ipp::operation::{IppOperation, PrintJob};
use ipp::prelude::*;
use thiserror::Error;

use crate::domain::job::JobDetail;
use crate::domain::print_options::{DocumentFormat, PrintJobOptions};
use crate::domain::printer::{PrinterCapabilities, PrinterInfo};

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

    #[error("format '{requested}' not supported; printer accepts: {}", supported.join(", "))]
    FormatNotSupported {
        requested: String,
        supported: Vec<String>,
    },

    #[error("image conversion failed: {0}")]
    ConversionError(String),
}

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

    // The ipp crate requires http:// — it speaks IPP-over-HTTP, not HTTPS.
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

    pub async fn get_printer_capabilities(
        &self,
        printer: &str,
    ) -> Result<PrinterCapabilities, CupsError> {
        let uri = self.printer_uri(printer)?;
        let op = IppOperationBuilder::get_printer_attributes(uri.clone())
            .attributes([
                "printer-make-and-model",
                "printer-state",
                "printer-state-reasons",
                "document-format-supported",
                "media-supported",
                "media-default",
                "media-ready",
                "sides-supported",
                "sides-default",
                "print-color-mode-supported",
                "print-color-mode-default",
            ])
            .build();
        let client = AsyncIppClient::new(uri);
        let resp = client.send(op).await?;

        if resp.header().status_code() == StatusCode::ClientErrorNotFound {
            return Err(CupsError::PrinterNotFound(printer.to_owned()));
        }
        assert_success(&resp)?;

        let mut caps = PrinterCapabilities {
            make_and_model: String::new(),
            state: "unknown".to_owned(),
            state_reasons: Vec::new(),
            formats_supported: Vec::new(),
            media_supported: Vec::new(),
            media_default: String::new(),
            media_ready: Vec::new(),
            sides_supported: Vec::new(),
            sides_default: String::new(),
            color_modes_supported: Vec::new(),
            color_mode_default: String::new(),
        };

        for group in resp.attributes().groups_of(DelimiterTag::PrinterAttributes) {
            let attrs = group.attributes();

            if let Some(a) = attrs.get("printer-make-and-model") {
                caps.make_and_model = ipp_keyword(a.value());
            }
            if let Some(a) = attrs.get("printer-state") {
                caps.state = printer_state_str(a.value());
            }
            if let Some(a) = attrs.get("printer-state-reasons") {
                caps.state_reasons = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("document-format-supported") {
                caps.formats_supported = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("media-supported") {
                caps.media_supported = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("media-default") {
                caps.media_default = ipp_keyword(a.value());
            }
            if let Some(a) = attrs.get("media-ready") {
                caps.media_ready = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("sides-supported") {
                caps.sides_supported = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("sides-default") {
                caps.sides_default = ipp_keyword(a.value());
            }
            if let Some(a) = attrs.get("print-color-mode-supported") {
                caps.color_modes_supported = ipp_keywords(a.value());
            }
            if let Some(a) = attrs.get("print-color-mode-default") {
                caps.color_mode_default = ipp_keyword(a.value());
            }
        }

        Ok(caps)
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
        format: &DocumentFormat,
        job_name: Option<&str>,
        options: &PrintJobOptions,
    ) -> Result<i32, CupsError> {
        let uri = self.printer_uri(printer)?;
        let payload = IppPayload::new(std::io::Cursor::new(data));

        // Convert to a raw request so we can inject document-format into
        // OperationAttributes — the high-level PrintJob builder doesn't expose it.
        let op = PrintJob::new(uri.clone(), payload, None::<&str>, job_name);
        let mut req = op.into_ipp_request();
        req.attributes_mut().add(
            DelimiterTag::OperationAttributes,
            IppAttribute::new(
                "document-format",
                IppValue::MimeMediaType(format.mime_type().to_owned()),
            ),
        );

        if let Some(copies) = options.copies {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new(
                    "copies",
                    IppValue::Integer(i32::try_from(copies).unwrap_or(i32::MAX)),
                ),
            );
        }
        if let Some(media) = &options.media {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new(
                    "media",
                    IppValue::Keyword(media.as_cups_keyword().to_owned()),
                ),
            );
        }
        if let Some(sides) = &options.sides {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new(
                    "sides",
                    IppValue::Keyword(sides.as_ipp_keyword().to_owned()),
                ),
            );
        }
        if let Some(color_mode) = &options.color_mode {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new(
                    "print-color-mode",
                    IppValue::Keyword(color_mode.as_ipp_keyword().to_owned()),
                ),
            );
        }
        if let Some(orientation) = &options.orientation {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new(
                    "orientation-requested",
                    IppValue::Enum(orientation.as_ipp_enum()),
                ),
            );
        }

        // For images sent to page-based printers, prevent CUPS from centering the
        // image in the middle of the sheet. `print-scaling=none` prints at native
        // size starting from the top-left origin of the printable area.
        if matches!(format, DocumentFormat::Jpeg | DocumentFormat::Png) {
            req.attributes_mut().add(
                DelimiterTag::JobAttributes,
                IppAttribute::new("print-scaling", IppValue::Keyword("none".to_owned())),
            );
        }

        // Request zero margins for PDF jobs so the content starts from the top-left
        // of the physical paper rather than after the printer's default margin offset.
        if matches!(format, DocumentFormat::Pdf) {
            for attr in [
                "media-top-margin",
                "media-left-margin",
                "media-right-margin",
                "media-bottom-margin",
            ] {
                req.attributes_mut().add(
                    DelimiterTag::JobAttributes,
                    IppAttribute::new(attr, IppValue::Integer(0)),
                );
            }
        }

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

        // No high-level GetJobs builder in ipp v4 — construct the IPP request manually.
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

fn assert_success(resp: &IppRequestResponse) -> Result<(), CupsError> {
    let code = resp.header().status_code();
    if code.is_success() {
        Ok(())
    } else {
        Err(CupsError::IppStatus(format!("{code:?}")))
    }
}

fn printer_state_str(value: &IppValue) -> String {
    match value {
        IppValue::Enum(3) => "idle".to_owned(),
        IppValue::Enum(4) => "processing".to_owned(),
        IppValue::Enum(5) => "stopped".to_owned(),
        IppValue::Enum(n) => format!("unknown({n})"),
        _ => "unknown".to_owned(),
    }
}

/// Extract all keyword/text values from a scalar or Array IPP value.
fn ipp_keywords(value: &IppValue) -> Vec<String> {
    match value {
        IppValue::Array(items) => items.iter().map(ipp_keyword).collect(),
        other => vec![ipp_keyword(other)],
    }
}

fn ipp_keyword(value: &IppValue) -> String {
    match value {
        IppValue::Keyword(s)
        | IppValue::MimeMediaType(s)
        | IppValue::NameWithoutLanguage(s)
        | IppValue::TextWithoutLanguage(s) => s.clone(),
        other => format!("{other:?}"),
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
