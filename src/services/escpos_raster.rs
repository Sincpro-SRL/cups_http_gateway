use image::GrayImage;
use tracing::{debug, info, warn};

/// Converts a JPEG or PNG image to an ESC/POS raster bit-image (`GS v 0`) stream.
///
/// Pipeline: decode → scale to `width_px` → grayscale → Floyd-Steinberg → pack MSB-first.
/// On failure the original bytes are returned unchanged with a warning.
pub fn to_escpos_raster(data: &[u8], width_px: u32) -> Vec<u8> {
    let img = match image::load_from_memory(data) {
        Ok(img) => img,
        Err(e) => {
            warn!(error = %e, "raster: image decode failed — sending original");
            return data.to_vec();
        }
    };

    let scaled = if img.width() == width_px {
        img
    } else {
        let resized = img.resize(width_px, u32::MAX, image::imageops::FilterType::Lanczos3);
        info!(
            from_px = img.width(),
            to_px = resized.width(),
            height_px = resized.height(),
            "raster: image scaled"
        );
        resized
    };

    let gray: GrayImage = scaled.to_luma8();
    let w = gray.width();
    let h = gray.height();

    debug!(
        width = w,
        height = h,
        "raster: starting Floyd-Steinberg dither"
    );
    let bits = floyd_steinberg(&gray);

    let bytes_per_line = w.div_ceil(8);
    let raster = pack_bits(&bits, w, h, bytes_per_line);

    let mut out = Vec::with_capacity(8 + raster.len());

    // GS v 0: [0x1D, 0x76, 0x30, m=0 (normal/203 DPI), xL, xH, yL, yH, ...data]
    out.extend_from_slice(&[
        0x1D,
        0x76,
        0x30,
        0x00,
        (bytes_per_line & 0xFF) as u8,
        ((bytes_per_line >> 8) & 0xFF) as u8,
        (h & 0xFF) as u8,
        ((h >> 8) & 0xFF) as u8,
    ]);
    out.extend_from_slice(&raster);

    info!(
        bytes = out.len(),
        width_px = w,
        height_px = h,
        bytes_per_line,
        "raster: ESC/POS GS v 0 image ready"
    );
    out
}

/// Floyd-Steinberg error-diffusion dithering. Returns a flat 0/1 vec (0=black), row-major.
fn floyd_steinberg(gray: &GrayImage) -> Vec<u8> {
    let w = gray.width() as usize;
    let h = gray.height() as usize;

    // i32 to allow error accumulation without per-pixel clamping.
    let mut buf: Vec<i32> = gray.pixels().map(|p| i32::from(p.0[0])).collect();

    for y in 0..h {
        for x in 0..w {
            let old = buf[y * w + x].clamp(0, 255);
            let new = if old > 128 { 255 } else { 0 };
            buf[y * w + x] = new;

            let err = old - new;
            if err == 0 {
                continue;
            }

            if x + 1 < w {
                buf[y * w + x + 1] += err * 7 / 16;
            }
            if y + 1 < h {
                if x > 0 {
                    buf[(y + 1) * w + x - 1] += err * 3 / 16;
                }
                buf[(y + 1) * w + x] += err * 5 / 16;
                if x + 1 < w {
                    buf[(y + 1) * w + x + 1] += err / 16;
                }
            }
        }
    }

    buf.iter().map(|v| u8::from(*v > 128)).collect()
}

/// Packs 1-bit pixels MSB-first into bytes (ESC/POS: bit 7 = leftmost pixel).
fn pack_bits(bits: &[u8], w: u32, h: u32, bytes_per_line: u32) -> Vec<u8> {
    let w = w as usize;
    let h = h as usize;
    let bpl = bytes_per_line as usize;
    let mut out = vec![0u8; bpl * h];

    for y in 0..h {
        for x in 0..w {
            // 0 = black in dither buf; ESC/POS bit=1 means "print dot".
            if bits[y * w + x] == 0 {
                let byte_idx = y * bpl + x / 8;
                let bit_idx = 7 - (x % 8); // MSB first
                out[byte_idx] |= 1 << bit_idx;
            }
        }
    }
    out
}
