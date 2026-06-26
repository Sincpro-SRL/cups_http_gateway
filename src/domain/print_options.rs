/// Typed MIME format. Use `Raw` for anything CUPS supports that isn't listed here.
#[derive(Debug, Clone)]
pub enum DocumentFormat {
    PlainText,
    Pdf,
    PostScript,
    Png,
    Jpeg,
    /// Thermal receipt printers (ESC/POS), label printers (ZPL), or any custom MIME.
    Raw(String),
}

impl DocumentFormat {
    pub fn mime_type(&self) -> &str {
        match self {
            Self::PlainText => "text/plain",
            Self::Pdf => "application/pdf",
            Self::PostScript => "application/postscript",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Raw(mime) => mime,
        }
    }

    pub fn from_mime(s: &str) -> Self {
        match s {
            "text/plain" => Self::PlainText,
            "application/pdf" => Self::Pdf,
            "application/postscript" => Self::PostScript,
            "image/png" => Self::Png,
            "image/jpeg" | "image/jpg" => Self::Jpeg,
            other => Self::Raw(other.to_owned()),
        }
    }

    /// Whether the content arrives as plain UTF-8 (no base64 needed).
    pub fn is_text(&self) -> bool {
        matches!(self, Self::PlainText)
    }
}

/// Duplex / sides selection.
#[derive(Debug, Clone, Default)]
pub enum Sides {
    #[default]
    OneSided,
    TwoSidedLongEdge,
    TwoSidedShortEdge,
}

impl Sides {
    pub fn as_ipp_keyword(&self) -> &'static str {
        match self {
            Self::OneSided => "one-sided",
            Self::TwoSidedLongEdge => "two-sided-long-edge",
            Self::TwoSidedShortEdge => "two-sided-short-edge",
        }
    }

    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "one-sided" => Some(Self::OneSided),
            "two-sided-long-edge" => Some(Self::TwoSidedLongEdge),
            "two-sided-short-edge" => Some(Self::TwoSidedShortEdge),
            _ => None,
        }
    }
}

/// Color vs monochrome.
#[derive(Debug, Clone)]
pub enum ColorMode {
    Color,
    Monochrome,
    Auto,
}

impl ColorMode {
    pub fn as_ipp_keyword(&self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Monochrome => "monochrome",
            Self::Auto => "auto",
        }
    }

    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "color" => Some(Self::Color),
            "monochrome" | "grayscale" => Some(Self::Monochrome),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Page orientation.
#[derive(Debug, Clone)]
pub enum Orientation {
    Portrait,
    Landscape,
    ReversePortrait,
    ReverseLandscape,
}

impl Orientation {
    /// IPP enum value for `orientation-requested`.
    pub fn as_ipp_enum(&self) -> i32 {
        match self {
            Self::Portrait => 3,
            Self::Landscape => 4,
            Self::ReversePortrait => 5,
            Self::ReverseLandscape => 6,
        }
    }

    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "portrait" => Some(Self::Portrait),
            "landscape" => Some(Self::Landscape),
            "reverse-portrait" => Some(Self::ReversePortrait),
            "reverse-landscape" => Some(Self::ReverseLandscape),
            _ => None,
        }
    }
}

/// Paper / label size forwarded to CUPS as a media keyword.
///
/// Use `Custom` for thermal receipt rolls (`custom_80x297mm`, `custom_58x297mm`)
/// or any size not listed here. The string must be a valid CUPS media keyword.
#[derive(Debug, Clone)]
pub enum MediaSize {
    // ── ISO ───────────────────────────────────────────────────────────────────
    A3,
    A4,
    A5,
    A6,
    // ── North America ─────────────────────────────────────────────────────────
    Letter,
    Legal,
    Executive,
    // ── Labels & receipts ─────────────────────────────────────────────────────
    /// 4×6 in — common shipping / label format.
    Label4x6,
    /// 80 mm wide receipt roll (common thermal receipt printer).
    ThermalReceipt80mm,
    /// 58 mm wide receipt roll (compact thermal printer).
    ThermalReceipt58mm,
    /// Any CUPS media keyword not covered above, e.g. `"custom_100x150mm"`.
    Custom(String),
}

impl MediaSize {
    pub fn as_cups_keyword(&self) -> &str {
        match self {
            Self::A3 => "iso_a3",
            Self::A4 => "iso_a4",
            Self::A5 => "iso_a5",
            Self::A6 => "iso_a6",
            Self::Letter => "na_letter",
            Self::Legal => "na_legal",
            Self::Executive => "na_executive",
            Self::Label4x6 => "na_index-4x6",
            Self::ThermalReceipt80mm => "custom_80x297mm",
            Self::ThermalReceipt58mm => "custom_58x297mm",
            Self::Custom(s) => s,
        }
    }

    pub fn from_keyword(s: &str) -> Self {
        match s {
            "iso_a3" => Self::A3,
            "iso_a4" => Self::A4,
            "iso_a5" => Self::A5,
            "iso_a6" => Self::A6,
            "na_letter" => Self::Letter,
            "na_legal" => Self::Legal,
            "na_executive" => Self::Executive,
            "na_index-4x6" => Self::Label4x6,
            "custom_80x297mm" => Self::ThermalReceipt80mm,
            "custom_58x297mm" => Self::ThermalReceipt58mm,
            other => Self::Custom(other.to_owned()),
        }
    }
}

/// IPP job attributes forwarded to CUPS with each print job.
///
/// All fields are optional — omitted fields use the printer's configured defaults.
#[derive(Debug, Clone, Default)]
pub struct PrintJobOptions {
    pub copies: Option<u32>,
    pub media: Option<MediaSize>,
    pub sides: Option<Sides>,
    pub color_mode: Option<ColorMode>,
    pub orientation: Option<Orientation>,
}
