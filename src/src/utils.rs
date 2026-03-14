// ─── Mojibake fix ─────────────────────────────────────────────────────────────

/// OBS on Windows sometimes writes UTF-8 text that was saved as Latin-1.
/// e.g. "JÃ¡" → "Já"
pub fn fix_mojibake(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    // Only attempt if all chars fit in a single byte (Latin-1 range)
    if s.chars().all(|c| c as u32 <= 0xFF) {
        let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
        if let Ok(decoded) = std::str::from_utf8(&bytes) {
            // Reject if UTF-8 replacement characters appeared (garbage)
            if !decoded.contains('\u{FFFD}') {
                return decoded.to_string();
            }
        }
    }
    s.to_string()
}

// ─── Color conversion ─────────────────────────────────────────────────────────

/// OBS stores colors as ABGR integers (little-endian: R G B A).
/// Converts to CSS rgba().
/// opacity_pct: 0–100, applied on top of the alpha channel.
pub fn obs_color_to_css(color: Option<i64>, opacity_pct: f64) -> String {
    match color {
        None => format!("rgba(255,255,255,{:.3})", opacity_pct / 100.0),
        Some(c) => {
            let c = c as u32; // wraps correctly for both signed/unsigned OBS values
            let r = c & 0xFF;
            let g = (c >> 8) & 0xFF;
            let b = (c >> 16) & 0xFF;
            let a = (c >> 24) & 0xFF;
            let alpha = (a as f64 / 255.0) * (opacity_pct / 100.0);
            format!("rgba({},{},{},{:.3})", r, g, b, alpha)
        }
    }
}

// ─── Outline shadow ───────────────────────────────────────────────────────────

/// Build a CSS text-shadow string that simulates OBS text outline.
/// `size_px` is already in CSS pixels (caller divides by item scale so the
/// visual result on the canvas matches the OBS outline_size value).
/// 12 evenly-spaced directions (30° step) produce a smooth circular halo.
pub fn build_outline_shadow(size_px: f64, color: Option<i64>) -> String {
    let s = size_px.clamp(0.5, 20.0);
    let color_css = obs_color_to_css(color, 100.0);
    // 12 directions at 30° intervals
    let steps = 12usize;
    (0..steps)
        .map(|i| {
            let angle = (i as f64) * std::f64::consts::TAU / (steps as f64);
            let x = s * angle.cos();
            let y = s * angle.sin();
            format!("{:.2}px {:.2}px 0 {}", x, y, color_css)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ─── Font map ─────────────────────────────────────────────────────────────────

pub struct FontMapping {
    pub web:         &'static str,
    pub google_font: Option<&'static str>,
}

/// Maps Windows system font names → web-safe equivalents + Google Fonts ID.
pub fn get_web_font(face: &str) -> Option<FontMapping> {
    match face {
        "OCR A Extended"      => Some(FontMapping { web: "Share Tech Mono",                    google_font: Some("Share+Tech+Mono") }),
        "Comic Sans MS"       => Some(FontMapping { web: "Indie Flower",                       google_font: Some("Indie+Flower") }),
        "Rockwell Extra Bold" => Some(FontMapping { web: "Alfa Slab One",                      google_font: Some("Alfa+Slab+One") }),
        "Bangers"             => Some(FontMapping { web: "Bangers",                            google_font: Some("Bangers") }),
        "Impact"              => Some(FontMapping { web: "Anton",                              google_font: Some("Anton") }),
        "Arial"               => Some(FontMapping { web: "Arial, sans-serif",                  google_font: None }),
        "Segoe UI"            => Some(FontMapping { web: "'Segoe UI', system-ui, sans-serif",  google_font: None }),
        _                     => None,
    }
}
