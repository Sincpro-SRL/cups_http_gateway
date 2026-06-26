use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::adapters::cups::client::CupsError;
use crate::adapters::cups::CupsClient;
use crate::domain::job::JobDetail;
use crate::domain::print_options::{ColorMode, DocumentFormat, MediaSize, PrintJobOptions, Sides};
use crate::domain::printer::{PrinterCapabilities, PrinterInfo};
use crate::services::capabilities_cache::CapabilitiesCache;
use crate::services::escpos_raster;

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

    pub async fn submit_job(
        &self,
        printer: &str,
        mut data: Vec<u8>,
        mut format: DocumentFormat,
        job_name: Option<&str>,
        mut options: PrintJobOptions,
    ) -> Result<i32, CupsError> {
        let caps = self.printer_capabilities(printer).await?;

        // Hard error: we cannot transcode content, so reject unsupported formats up front.
        let mime = format.mime_type();
        if !caps.formats_supported.is_empty()
            && !caps.formats_supported.iter().any(|f| f == mime)
            // application/octet-stream is universally accepted for raw streams
            && mime != "application/octet-stream"
        {
            return Err(CupsError::FormatNotSupported {
                requested: mime.to_owned(),
                supported: caps.formats_supported.clone(),
            });
        }

        resolve_options(&mut options, &caps, printer);

        if matches!(format, DocumentFormat::Jpeg | DocumentFormat::Png) {
            // Thermal detection: check CUPS caps first, then fall back to the
            // requested media — the printer may be misconfigured in CUPS (e.g. letter
            // as default) even though it is physically a thermal roll printer.
            let target_px = thermal_width_px(&caps).or_else(|| {
                options
                    .media
                    .as_ref()
                    .and_then(MediaSize::thermal_print_width_px)
            });

            if let Some(target_px) = target_px {
                debug!(
                    printer = printer,
                    target_px, "thermal path — converting image to ESC/POS raster"
                );
                data = escpos_raster::to_escpos_raster(&data, target_px);
                format = DocumentFormat::Raw("application/octet-stream".to_owned());
            }
        }

        info!(
            printer = printer,
            format = format.mime_type(),
            bytes = data.len(),
            job_name = job_name.unwrap_or("(none)"),
            copies = ?options.copies,
            media = ?options.media.as_ref().map(MediaSize::as_cups_keyword),
            "submitting print job"
        );

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

// ── Thermal detection ─────────────────────────────────────────────────────────

// 203 DPI is the standard resolution for thermal receipt printers (ESC/POS spec).
const THERMAL_DPI: u32 = 203;

/// Returns the printable width in pixels for thermal printers, or `None` for
/// standard page-based printers.
///
/// Thermal detection: `media_default` (or any `media_ready` entry) that matches
/// the `custom_NNxMMMmm` pattern with width ≤ 120mm is treated as a thermal roll.
/// This covers 58mm, 76mm, 80mm, and similar receipt sizes.
fn thermal_width_px(caps: &PrinterCapabilities) -> Option<u32> {
    let width_mm = detect_thermal_media(caps)?;
    // Printable area = 90% of total roll width (standard 5% margins each side).
    // u32→f64 is lossless; f64→u32 is bounded by paper widths so safe.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((f64::from(width_mm) * 0.9 / 25.4 * f64::from(THERMAL_DPI)).round() as u32)
}

/// Returns the roll width in mm if the printer's active media is a thermal receipt
/// roll, or `None` if it looks like a standard page-based printer.
fn detect_thermal_media(caps: &PrinterCapabilities) -> Option<u32> {
    // media_ready = physically loaded; fall back to media_default if empty.
    let candidates = caps
        .media_ready
        .iter()
        .chain(std::iter::once(&caps.media_default));

    for kw in candidates {
        if let Some(mm) = parse_roll_width_mm(kw) {
            return Some(mm);
        }
    }
    None
}

/// Parses `custom_NNxMMMmm` → roll width in mm, only for widths ≤ 120mm
/// (anything wider is likely a label printer or cut-sheet, not a receipt roll).
fn parse_roll_width_mm(keyword: &str) -> Option<u32> {
    let inner = keyword.strip_prefix("custom_")?;
    let width_str = inner.split('x').next()?;
    let width_mm: u32 = width_str.parse().ok()?;
    if width_mm <= 120 {
        Some(width_mm)
    } else {
        None
    }
}

// ── Option resolution ─────────────────────────────────────────────────────────

/// Fills unspecified options from the printer's CUPS defaults.
/// If the client specified a value not in the supported list, logs a warning
/// and falls back to the printer default (never errors — CUPS decides).
fn resolve_options(options: &mut PrintJobOptions, caps: &PrinterCapabilities, printer: &str) {
    options.media = resolve_keyword(
        options.media.as_ref().map(MediaSize::as_cups_keyword),
        &caps.media_supported,
        &caps.media_default,
        printer,
        "media",
    )
    .map(|kw| MediaSize::from_keyword(&kw));

    options.sides = resolve_keyword(
        options.sides.as_ref().map(Sides::as_ipp_keyword),
        &caps.sides_supported,
        &caps.sides_default,
        printer,
        "sides",
    )
    .and_then(|kw| Sides::from_keyword(&kw));

    options.color_mode = resolve_keyword(
        options.color_mode.as_ref().map(ColorMode::as_ipp_keyword),
        &caps.color_modes_supported,
        &caps.color_mode_default,
        printer,
        "color_mode",
    )
    .and_then(|kw| ColorMode::from_keyword(&kw));
}

fn resolve_keyword(
    requested: Option<&str>,
    supported: &[String],
    default_kw: &str,
    printer: &str,
    field: &str,
) -> Option<String> {
    let candidate = match requested {
        Some(kw) if supported.is_empty() || supported.iter().any(|s| s == kw) => {
            return Some(kw.to_owned());
        }
        Some(kw) => {
            warn!(
                printer,
                field,
                requested = kw,
                fallback = default_kw,
                "option not supported — using printer default"
            );
            default_kw
        }
        None => default_kw,
    };

    if candidate.is_empty() {
        return None;
    }
    if supported.is_empty() || supported.iter().any(|s| s == candidate) {
        Some(candidate.to_owned())
    } else {
        None
    }
}
