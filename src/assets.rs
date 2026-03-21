use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::parser::{SceneData, SceneItem};
use crate::utils::{get_chroma_key, get_web_font};

// ─── Public API ───────────────────────────────────────────────────────────────

pub struct AssetResult {
    /// Maps original local file path → web-relative path (None if not found/failed)
    pub asset_map: HashMap<String, Option<String>>,
    pub font_css:  String,
}

pub async fn process_assets<F>(
    scene:      &SceneData,
    output_dir: &str,
    log:        F,
) -> AssetResult
where
    F: Fn(String) + Send,
{
    let mut asset_map: HashMap<String, Option<String>> = HashMap::new();

    // ── Copy local media files ───────────────────────────────────────────────
    let local_files    = collect_local_files(&scene.items);
    let chroma_videos  = collect_chroma_videos(&scene.items);
    let ffmpeg_ok      = find_ffmpeg();
    if !chroma_videos.is_empty() {
        if ffmpeg_ok {
            log("✓ ffmpeg found — chroma key videos will be converted to WebM alpha".to_string());
        } else {
            log("⚠ ffmpeg not found — chroma key will use canvas JS fallback. Install ffmpeg and add it to PATH for automatic conversion.".to_string());
        }
    }

    for file_path in &local_files {
        if file_path.is_empty() { continue; }

        let path = Path::new(file_path);

        if !path.exists() {
            log(format!("✗ Not found: {}", file_path));
            asset_map.insert(file_path.clone(), None);
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let sub_dir  = get_sub_dir(&ext);
        let dest_dir = PathBuf::from(output_dir).join("assets").join(sub_dir);

        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            log(format!("✗ Error creating folder: {}", e));
            asset_map.insert(file_path.clone(), None);
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let dest_path = dest_dir.join(filename);

        // Chroma key video → try ffmpeg WebM alpha conversion first
        let is_video = matches!(ext.as_str(), "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv");
        if is_video && ffmpeg_ok {
            if let Some(&(kr, kg, kb, sim)) = chroma_videos.get(file_path.as_str()) {
                let stem     = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
                let webm_name = format!("{}_alpha.webm", stem);
                let webm_dest = dest_dir.join(&webm_name);
                log(format!("⚙ Converting chroma key: {} → {} …", filename, webm_name));
                match convert_chroma_to_webm(path, &webm_dest, kr, kg, kb, sim) {
                    Ok(()) => {
                        let web_path = format!("assets/{}/{}", sub_dir, webm_name);
                        log(format!("✓ Converted: {}", webm_name));
                        asset_map.insert(file_path.clone(), Some(web_path));
                        continue;
                    }
                    Err(e) => {
                        log(format!("⚠ ffmpeg failed ({}), copying original", e));
                    }
                }
            }
        }

        match std::fs::copy(path, &dest_path) {
            Ok(_) => {
                let web_path = format!("assets/{}/{}", sub_dir, filename);
                log(format!("✓ Copied: {}", filename));
                asset_map.insert(file_path.clone(), Some(web_path));
            }
            Err(e) => {
                log(format!("✗ Error copying {}: {}", filename, e));
                asset_map.insert(file_path.clone(), None);
            }
        }
    }

    // ── Download Google Fonts ────────────────────────────────────────────────
    let google_fonts = collect_google_fonts(&scene.fonts);
    let mut font_css_blocks: Vec<String> = Vec::new();

    if !google_fonts.is_empty() {
        let fonts_dir = PathBuf::from(output_dir).join("assets").join("fonts");
        let _ = std::fs::create_dir_all(&fonts_dir);

        for (face, google_font) in &google_fonts {
            log(format!("↓ Downloading font: {} …", google_font));
            match download_font(google_font, &fonts_dir).await {
                Ok(css) => {
                    log(format!("✓ Font downloaded: {}", face));
                    font_css_blocks.push(css);
                }
                Err(e) => {
                    log(format!("⚠ Failed to download font ({}) — using CDN fallback", e));
                    font_css_blocks.push(format!(
                        "@import url('https://fonts.googleapis.com/css2?family={}&display=swap');",
                        google_font
                    ));
                }
            }
        }
    }

    AssetResult {
        asset_map,
        font_css: font_css_blocks.join("\n\n"),
    }
}

// ─── Font downloading ─────────────────────────────────────────────────────────

async fn download_font(google_font: &str, fonts_dir: &Path) -> Result<String, String> {
    let css_url = format!(
        "https://fonts.googleapis.com/css2?family={}&display=swap",
        google_font
    );

    let client = reqwest::Client::builder()
        // Modern UA → Google Fonts returns WOFF2 instead of EOT/TTF
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;

    let mut css = client
        .get(&css_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // Extract WOFF2 URLs (simple string search — no regex dependency needed)
    let mut woff2_urls: Vec<String> = Vec::new();
    let mut search_from = 0;
    while let Some(pos) = css[search_from..].find("https://fonts.gstatic.com/") {
        let abs = search_from + pos;
        if let Some(end) = css[abs..].find(".woff2") {
            let url = css[abs..abs + end + 6].to_string(); // includes ".woff2"
            if !woff2_urls.contains(&url) {
                woff2_urls.push(url);
            }
            search_from = abs + end + 6;
        } else {
            break;
        }
    }

    // Download each WOFF2 file and rewrite URL in the CSS
    for url in &woff2_urls {
        let filename = format!(
            "{}_{}",
            google_font.replace('+', "_"),
            url.rsplit('/').next().unwrap_or("font.woff2")
        );
        let dest = fonts_dir.join(&filename);

        let data = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        std::fs::write(&dest, &data).map_err(|e| e.to_string())?;

        // Replace remote URL with local path in CSS
        css = css.replace(url.as_str(), &format!("assets/fonts/{}", filename));
    }

    Ok(css.trim().to_string())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get_sub_dir(ext: &str) -> &'static str {
    match ext {
        "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv" => "videos",
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "svg" | "gif" => "images",
        "mp3" | "wav" | "ogg" | "flac" | "aac" => "audio",
        _ => "misc",
    }
}

fn collect_local_files(items: &[SceneItem]) -> HashSet<String> {
    let mut files = HashSet::new();
    for item in items {
        if let Some(file) = item.file() {
            if !file.is_empty() && !file.starts_with("http") {
                files.insert(file.to_string());
            }
        }
        if let SceneItem::Group { items: children, .. } = item {
            for f in collect_local_files(children) {
                files.insert(f);
            }
        }
    }
    files
}

/// Returns true if `ffmpeg` is available in PATH.
fn find_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Converts a video with chroma key to WebM VP9 with alpha channel using ffmpeg.
/// `obs_similarity` is on the OBS scale (1–1000).
fn convert_chroma_to_webm(
    input:          &Path,
    output:         &Path,
    kr: u8, kg: u8, kb: u8,
    obs_similarity: f64,
) -> Result<(), String> {
    let color_hex   = format!("{:02X}{:02X}{:02X}", kr, kg, kb);
    // OBS similarity 1-1000 → ffmpeg 0.1-0.8
    // OBS sim/1000 gives the normalized ratio; minimum 0.1 so low-sim configs still remove the key color
    let ffmpeg_sim  = (obs_similarity / 1000.0).clamp(0.1, 0.8);
    let blend       = (obs_similarity / 5000.0).clamp(0.0, 0.15);
    // format=yuva420p must be explicit at the end of the filter chain —
    // without it, libvpx-vp9 ignores the -pix_fmt flag and produces yuv420p (no alpha).
    let vf          = format!(
        "chromakey=color=0x{}:similarity={:.4}:blend={:.4},format=yuva420p",
        color_hex, ffmpeg_sim, blend
    );

    let out = std::process::Command::new("ffmpeg")
        .args([
            "-i",    input.to_str().unwrap_or(""),
            "-vf",   &vf,
            "-c:v",  "libvpx-vp9",
            "-pix_fmt", "yuva420p",
            "-auto-alt-ref", "0",
            "-b:v",  "0",
            "-crf",  "30",
            "-an",
            output.to_str().unwrap_or(""),
            "-y",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(stderr.lines().last().unwrap_or("ffmpeg error").to_string())
    }
}

/// Builds a map of local file path → (R, G, B, obs_similarity) for videos with chroma key.
fn collect_chroma_videos(items: &[SceneItem]) -> HashMap<&str, (u8, u8, u8, f64)> {
    let mut result = HashMap::new();
    for item in items {
        if let SceneItem::Video { file, base, .. } = item {
            if let Some((r, g, b, sim)) = get_chroma_key(&base.filters) {
                result.insert(file.as_str(), (r, g, b, sim));
            }
        }
        if let SceneItem::Group { items: children, .. } = item {
            result.extend(collect_chroma_videos(children));
        }
    }
    result
}

fn collect_google_fonts(font_faces: &HashSet<String>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for face in font_faces {
        if let Some(mapping) = get_web_font(face) {
            if let Some(gf) = mapping.google_font {
                result.insert(face.clone(), gf.to_string());
            }
        }
    }
    result
}
