use std::io::{BufWriter, Cursor};

use printpdf::{ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px};

use crate::domain::print_options::MediaSize;

const REF_DPI: f32 = 96.0;

/// Wraps a PNG or JPEG in a single-page PDF anchored at the top-left corner.
/// If `target_w_mm` is given, the image is scaled to that physical width (e.g. 72mm
/// for an 80mm receipt). Otherwise it renders at native 96-DPI size, only scaling
/// down if it would overflow the page.
pub fn to_pdf(
    data: &[u8],
    media: Option<&MediaSize>,
    target_w_mm: Option<f32>,
) -> Result<Vec<u8>, String> {
    let dyn_img = image::load_from_memory(data).map_err(|e| e.to_string())?;
    let width = dyn_img.width();
    let height = dyn_img.height();

    // Decode to RGB8 — avoids the image version mismatch with printpdf's embedded_images.
    let rgb = dyn_img.into_rgb8();
    let xobj = ImageXObject {
        width: Px(width as usize),
        height: Px(height as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: rgb.into_raw(),
        image_filter: None,
        smask: None,
        clipping_bbox: None,
    };

    let (page_w, page_h) = media_dims_mm(media);
    let (doc, page1, layer1) = PdfDocument::new("print", Mm(page_w), Mm(page_h), "Layer 1");
    let layer = doc.get_page(page1).get_layer(layer1);

    #[allow(clippy::cast_precision_loss)]
    let (img_w, img_h) = (width as f32, height as f32);
    let natural_w = img_w / REF_DPI * 25.4;
    let natural_h = img_h / REF_DPI * 25.4;

    // Scale to the client's intended physical width if provided, otherwise native size.
    // Always clamp so the image cannot overflow the page in either dimension.
    let scale = target_w_mm
        .map_or(1.0, |tw| tw / natural_w)
        .min(page_w / natural_w)
        .min(page_h / natural_h);

    let rendered_h = natural_h * scale;

    // PDF origin is bottom-left; translate so the image sits flush at the top.
    let y = page_h - rendered_h;

    Image::from(xobj).add_to_layer(
        layer,
        ImageTransform {
            translate_x: Some(Mm(0.0)),
            translate_y: Some(Mm(y)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            dpi: Some(REF_DPI),
            rotate: None,
        },
    );

    let mut buf = BufWriter::new(Cursor::new(Vec::new()));
    doc.save(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf.into_inner().map_err(|e| e.to_string())?.into_inner())
}

fn media_dims_mm(media: Option<&MediaSize>) -> (f32, f32) {
    match media {
        Some(MediaSize::A3) => (297.0, 420.0),
        Some(MediaSize::A5) => (148.0, 210.0),
        Some(MediaSize::A6) => (105.0, 148.0),
        Some(MediaSize::Letter) => (215.9, 279.4),
        Some(MediaSize::Legal) => (215.9, 355.6),
        Some(MediaSize::Executive) => (184.2, 266.7),
        Some(MediaSize::Label4x6) => (101.6, 152.4),
        _ => (210.0, 297.0), // A4 for A4, thermal variants, unknown, or None
    }
}
