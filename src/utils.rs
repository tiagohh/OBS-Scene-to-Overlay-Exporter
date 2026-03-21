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

// ─── Chroma key ───────────────────────────────────────────────────────────────

/// Returns `(R, G, B, obs_similarity)` for the first chroma/color-key filter found.
/// `obs_similarity` is on the OBS scale (1–1000).
/// Returns None for black color keys (handled via mix-blend-mode: screen).
pub fn get_chroma_key(filters: &[crate::parser::Filter]) -> Option<(u8, u8, u8, f64)> {
    for f in filters {
        let s   = &f.settings;
        let sim = s["similarity"].as_f64().unwrap_or(80.0).clamp(1.0, 1000.0);
        match f.id.as_str() {
            "chroma_key_filter" | "chroma_key_filter_v2" => {
                let (r, g, b) = match s["key_color_type"].as_str().unwrap_or("green") {
                    "blue"    => (0u8, 0u8, 255u8),
                    "magenta" => (255u8, 0u8, 255u8),
                    "custom"  => {
                        let c = s["custom_color"].as_i64().unwrap_or(0) as u32;
                        ((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
                    }
                    _ => (0u8, 255u8, 0u8), // green
                };
                return Some((r, g, b, sim));
            }
            "color_key_filter" | "color_key_filter_v2" => {
                let c = s["color"].as_i64().unwrap_or(0) as u32;
                let r = (c & 0xFF) as u8;
                let g = ((c >> 8) & 0xFF) as u8;
                let b = ((c >> 16) & 0xFF) as u8;
                if r < 10 && g < 10 && b < 10 {
                    return None; // Black key → mix-blend-mode: screen
                }
                return Some((r, g, b, sim));
            }
            _ => {}
        }
    }
    None
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
