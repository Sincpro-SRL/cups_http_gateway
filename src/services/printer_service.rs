use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::adapters::cups::client::CupsError;
use crate::adapters::cups::CupsClient;
use crate::domain::job::JobDetail;
use crate::domain::print_options::{ColorMode, DocumentFormat, MediaSize, PrintJobOptions, Sides};
use crate::domain::printer::{PrinterCapabilities, PrinterInfo};
use crate::services::capabilities_cache::CapabilitiesCache;

/// Stateless service layer: validates input, delegates to the CUPS adapter.
/// Does not depend on any HTTP primitives — can be used by any transport layer.
pub struct PrinterService {
    cups: CupsClient,
    caps_cache: Arc<CapabilitiesCache>,
}

impl PrinterService {
    /// `caps_ttl` — how long to cache printer capabilities before re-querying CUPS.
    /// Pass `Duration::ZERO` to disable caching entirely.
    pub fn new(cups: CupsClient, caps_ttl: Duration) -> Self {
        Self {
            cups,
            caps_cache: Arc::new(CapabilitiesCache::new(caps_ttl)),
        }
    }

    pub async fn list_printers(&self) -> Result<Vec<String>, CupsError> {
        debug!("listing printers");
        let printers = self.cups.get_printers().await?;
        info!(count = printers.len(), "printers listed");
        Ok(printers)
    }

    pub async fn printer_capabilities(&self, name: &str) -> Result<PrinterCapabilities, CupsError> {
        if let Some(cached) = self.caps_cache.get(name).await {
            debug!(printer = name, "capabilities served from cache");
            return Ok(cached);
        }
        debug!(printer = name, "fetching printer capabilities from CUPS");
        let caps = self.cups.get_printer_capabilities(name).await?;
        info!(
            printer = name,
            formats = caps.formats_supported.len(),
            media_default = %caps.media_default,
            "printer capabilities fetched and cached"
        );
        self.caps_cache.set(name, caps.clone()).await;
        Ok(caps)
    }

    pub async fn printer_info(&self, name: &str) -> Result<PrinterInfo, CupsError> {
        debug!(printer = name, "fetching printer info");
        let info = self.cups.get_printer_info(name).await?;
        debug!(printer = name, state = %info.state, queued = info.queued_jobs, "printer info fetched");
        Ok(info)
    }

    /// Submit a print job.
    ///
    /// `data` is raw bytes. Callers are responsible for encoding/decoding
    /// (e.g. base64 decode at the HTTP layer before calling this).
    /// When `options.smart` is true, capabilities are queried (from cache when
    /// available) and unsupported options fall back to printer defaults automatically.
    pub async fn submit_job(
        &self,
        printer: &str,
        mut data: Vec<u8>,
        format: DocumentFormat,
        job_name: Option<&str>,
        mut options: PrintJobOptions,
    ) -> Result<i32, CupsError> {
        if options.smart {
            let caps = self.printer_capabilities(printer).await?;
            resolve_options(&format, &mut options, &caps, printer)?;
        }

        info!(
            printer = printer,
            format = format.mime_type(),
            bytes = data.len(),
            job_name = job_name.unwrap_or("(none)"),
            copies = ?options.copies,
            media = ?options.media.as_ref().map(MediaSize::as_cups_keyword),
            smart = options.smart,
            "submitting print job"
        );

        if let Some(cut) = &options.cut {
            if format.is_raw() {
                debug!(printer = printer, "appending ESC/POS cut bytes");
                data.extend_from_slice(cut.as_escpos_bytes());
            } else {
                warn!(
                    printer = printer,
                    format = format.mime_type(),
                    "cut requested but format is not raw — ignoring"
                );
            }
        }

        let job_id = self
            .cups
            .print_job(printer, data, &format, job_name, &options)
            .await?;
        info!(printer = printer, job_id = job_id, "print job accepted");
        Ok(job_id)
    }

    pub async fn list_jobs(&self, printer: &str) -> Result<Vec<JobDetail>, CupsError> {
        debug!(printer = printer, "listing jobs");
        let jobs = self.cups.get_jobs(printer).await?;
        debug!(printer = printer, count = jobs.len(), "jobs listed");
        Ok(jobs)
    }
}

// ── Smart option resolution ───────────────────────────────────────────────────

/// Validates and adjusts `options` against the printer's declared capabilities.
///
/// - Format not supported → hard error with the supported list.
/// - Option specified but not supported → falls back to the printer's CUPS default.
/// - Option not specified → filled in from the printer's CUPS default.
fn resolve_options(
    format: &DocumentFormat,
    options: &mut PrintJobOptions,
    caps: &PrinterCapabilities,
    printer: &str,
) -> Result<(), CupsError> {
    // ── Format (hard error — we cannot transcode) ─────────────────────────────
    let mime = format.mime_type();
    if !caps.formats_supported.is_empty() && !caps.formats_supported.iter().any(|f| f == mime) {
        return Err(CupsError::FormatNotSupported {
            requested: mime.to_owned(),
            supported: caps.formats_supported.clone(),
        });
    }

    // ── Media ─────────────────────────────────────────────────────────────────
    options.media = resolve_keyword_option(
        options.media.as_ref().map(MediaSize::as_cups_keyword),
        &caps.media_supported,
        &caps.media_default,
        printer,
        "media",
    )
    .map(|kw| MediaSize::from_keyword(&kw));

    // ── Sides ─────────────────────────────────────────────────────────────────
    options.sides = resolve_keyword_option(
        options.sides.as_ref().map(Sides::as_ipp_keyword),
        &caps.sides_supported,
        &caps.sides_default,
        printer,
        "sides",
    )
    .and_then(|kw| Sides::from_keyword(&kw));

    // ── Color mode ────────────────────────────────────────────────────────────
    options.color_mode = resolve_keyword_option(
        options.color_mode.as_ref().map(ColorMode::as_ipp_keyword),
        &caps.color_modes_supported,
        &caps.color_mode_default,
        printer,
        "color_mode",
    )
    .and_then(|kw| ColorMode::from_keyword(&kw));

    Ok(())
}

/// Resolves a single option keyword against the printer's supported list.
///
/// - `requested` is `Some(keyword)` when the caller specified a value.
/// - If not specified → use `default_keyword` from CUPS.
/// - If specified but not supported → fall back to `default_keyword` with a warning.
/// - Returns `None` if both the requested and the default are unsupported or empty.
fn resolve_keyword_option(
    requested: Option<&str>,
    supported: &[String],
    default_keyword: &str,
    printer: &str,
    field: &str,
) -> Option<String> {
    let candidate = match requested {
        Some(kw) => {
            if supported.is_empty() || supported.iter().any(|s| s == kw) {
                return Some(kw.to_owned());
            }
            warn!(
                printer = printer,
                field = field,
                requested = kw,
                fallback = default_keyword,
                "option not supported — falling back to printer default"
            );
            default_keyword
        }
        None => {
            // Not specified — apply the printer's configured default explicitly.
            default_keyword
        }
    };

    if candidate.is_empty() {
        return None;
    }
    if supported.is_empty() || supported.iter().any(|s| s == candidate) {
        debug!(
            printer = printer,
            field = field,
            value = candidate,
            "applying printer default"
        );
        Some(candidate.to_owned())
    } else {
        None
    }
}
