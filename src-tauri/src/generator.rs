use std::collections::HashMap;
use crate::parser::{Canvas, Filter, SceneData, SceneItem};
use crate::utils::{build_outline_shadow, get_web_font, obs_color_to_css};

// ─── Public API ───────────────────────────────────────────────────────────────

pub fn generate_html(
    scene:     &SceneData,
    asset_map: &HashMap<String, Option<String>>,
    font_css:  &str,
) -> String {
    let canvas = &scene.canvas;

    let layers: Vec<String> = scene
        .items
        .iter()
        .filter_map(|item| render_item(item, asset_map, canvas))
        .collect();
    let layers_html = layers.join("\n\n  ");

    let font_style = if font_css.is_empty() {
        String::new()
    } else {
        let indented: String = font_css
            .lines()
            .map(|l| format!("    {}", l))
            .collect::<Vec<_>>()
            .join("\n");
        format!("  <style>\n{}\n  </style>\n", indented)
    };

    // Double-brace {{ }} produces a literal { } in format! output
    format!(
        r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <title>{name}</title>
{font_style}  <style>
    *, *::before, *::after {{ margin: 0; padding: 0; box-sizing: border-box; }}

    html, body {{
      width: {cw}px;
      height: {ch}px;
      overflow: hidden;
      background: transparent;
    }}

    .canvas {{
      position: relative;
      width: {cw}px;
      height: {ch}px;
      overflow: hidden;
    }}

    /* Every layer is absolutely positioned, scaling from top-left — matches OBS align=5 */
    .layer {{
      position: absolute;
      transform-origin: top left;
    }}

    iframe.layer {{ border: none; background: transparent; }}
    video.layer  {{ display: block; }}
    img.layer    {{ display: block; }}
  </style>
</head>
<body>
  <div class="canvas">
  {layers}
  </div>
</body>
</html>"#,
        name       = scene.name,
        cw         = canvas.x,
        ch         = canvas.y,
        font_style = font_style,
        layers     = layers_html,
    )
}

// ─── Per-item renderer ────────────────────────────────────────────────────────

fn render_item(
    item:      &SceneItem,
    asset_map: &HashMap<String, Option<String>>,
    canvas:    &Canvas,
) -> Option<String> {
    match item {
        // ── Image ────────────────────────────────────────────────────────────
        SceneItem::Image { base, file } => {
            let style   = build_transform_style(base);
            let src     = resolve_asset(file, asset_map);
            let filters = build_css_filters(&base.filters);
            let extra   = if filters.is_empty() { String::new() } else { format!("; filter: {}", filters) };
            Some(format!(
                "<!-- {} (image) -->\n  <img id=\"{}\" class=\"layer\" src=\"{}\" alt=\"\" style=\"{}{}\">",
                base.name, base.id, src, style, extra
            ))
        }

        // ── GIF ──────────────────────────────────────────────────────────────
        SceneItem::Gif { base, file } => {
            let style = build_transform_style(base);
            let src   = resolve_asset(file, asset_map);
            Some(format!(
                "<!-- {} (gif) -->\n  <img id=\"{}\" class=\"layer\" src=\"{}\" alt=\"\" style=\"{}\">",
                base.name, base.id, src, style
            ))
        }

        // ── Video ─────────────────────────────────────────────────────────────
        SceneItem::Video { base, file, looping, .. } => {
            let style     = build_transform_style(base);
            let src       = resolve_asset(file, asset_map);
            let loop_attr = if *looping { " loop" } else { "" };
            let filters   = build_css_filters(&base.filters);
            let filter_str = if filters.is_empty() { String::new() } else { format!("; filter: {}", filters) };

            let has_chroma = base.filters.iter().any(|f| {
                matches!(
                    f.id.as_str(),
                    "chroma_key_filter" | "chroma_key_filter_v2" |
                    "color_key_filter"  | "color_key_filter_v2"
                )
            });
            let blend_str = if has_chroma { "; mix-blend-mode: screen" } else { "" };

            let ext  = file.rsplit('.').next().unwrap_or("").to_lowercase();
            let mime = if ext == "webm" { "video/webm" } else { "video/mp4" };

            Some(format!(
                "<!-- {} (video) -->\n  <video id=\"{}\" class=\"layer\" autoplay{} muted playsinline style=\"{}{}{}\">\n    <source src=\"{}\" type=\"{}\">\n  </video>",
                base.name, base.id, loop_attr, style, filter_str, blend_str, src, mime
            ))
        }

        // ── Audio — no visual element ────────────────────────────────────────
        SceneItem::Audio { .. } => None,

        // ── Text ──────────────────────────────────────────────────────────────
        SceneItem::Text {
            base, text, font_face, font_size, font_bold, font_italic,
            color, opacity, bk_color, bk_opacity,
            outline, outline_size, outline_color, drop_shadow,
            text_align, custom_width, ..
        } => {
            let style = build_transform_style(base);

            let font_family = match get_web_font(font_face) {
                Some(m) if m.google_font.is_some() =>
                    format!("'{}', '{}', monospace", m.web, font_face),
                Some(m) => m.web.to_string(),
                None    => format!("'{}', sans-serif", font_face),
            };

            let text_color = obs_color_to_css(*color, *opacity);

            let bg_css = if *bk_opacity > 0.0 {
                obs_color_to_css(*bk_color, *bk_opacity)
            } else {
                "transparent".to_string()
            };

            let mut shadows: Vec<String> = Vec::new();
            if *outline && *outline_size > 0 {
                shadows.push(build_outline_shadow(*outline_size, *outline_color));
            }
            if *drop_shadow {
                shadows.push("3px 3px 6px rgba(0,0,0,0.85)".to_string());
            }
            let text_shadow = if shadows.is_empty() { "none".to_string() } else { shadows.join(", ") };

            let width_css = if *custom_width > 0 {
                format!("; width: {}px; word-wrap: break-word", custom_width)
            } else {
                "; white-space: pre-line".to_string()
            };

            let full_style = format!(
                "{}; font-family: {}; font-size: {}px; font-weight: {}; font-style: {}; \
                 color: {}; background: {}; text-shadow: {}; text-align: {}; \
                 line-height: 1.25; padding: 4px 8px{}",
                style,
                font_family,
                font_size,
                if *font_bold   { "bold"   } else { "normal" },
                if *font_italic { "italic" } else { "normal" },
                text_color,
                bg_css,
                text_shadow,
                text_align,
                width_css,
            );

            let escaped = text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('\n', "<br>");

            Some(format!(
                "<!-- {} (text) -->\n  <div id=\"{}\" class=\"layer\" style=\"{}\">{}</div>",
                base.name, base.id, full_style, escaped
            ))
        }

        // ── Browser source (iframe) ───────────────────────────────────────────
        SceneItem::Browser { base, url, width, height } => {
            let style = build_transform_style(base);
            Some(format!(
                "<!-- {} (browser source) -->\n  <iframe id=\"{}\" class=\"layer\" src=\"{}\" width=\"{}\" height=\"{}\"\n    allow=\"autoplay\" style=\"{}; pointer-events: none;\"></iframe>",
                base.name, base.id, url, width, height, style
            ))
        }

        // ── Solid color ───────────────────────────────────────────────────────
        SceneItem::Color { base, color, width, height } => {
            let style = build_transform_style(base);
            let bg    = obs_color_to_css(*color, 100.0);
            Some(format!(
                "<!-- {} (color) -->\n  <div id=\"{}\" class=\"layer\" style=\"{}; width: {}px; height: {}px; background: {};\"></div>",
                base.name, base.id, style, width, height, bg
            ))
        }

        // ── Group / nested scene ──────────────────────────────────────────────
        SceneItem::Group { base, items } => {
            let style = build_transform_style(base);
            let children: Vec<String> = items
                .iter()
                .filter_map(|child| render_item(child, asset_map, canvas))
                .map(|html| {
                    html.lines()
                        .map(|l| format!("    {}", l))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .collect();
            Some(format!(
                "<!-- {} (group) -->\n  <div id=\"{}\" class=\"layer\" style=\"{}\">\n{}\n  </div>",
                base.name, base.id, style, children.join("\n\n")
            ))
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn build_transform_style(base: &crate::parser::BaseItem) -> String {
    let mut parts = vec![
        format!("left: {}px", base.pos.x),
        format!("top: {}px",  base.pos.y),
    ];

    let mut transforms = Vec::new();
    let sx = base.scale.x;
    let sy = base.scale.y;
    if (sx - sy).abs() < 0.0001 {
        transforms.push(format!("scale({:.4})", sx));
    } else {
        transforms.push(format!("scale({:.4}, {:.4})", sx, sy));
    }
    if base.rot.abs() > 0.001 {
        transforms.push(format!("rotate({:.2}deg)", base.rot));
    }
    if !transforms.is_empty() {
        parts.push(format!("transform: {}", transforms.join(" ")));
    }

    // bounds_type=2 → stretch to bounds → force explicit dimensions
    if base.bounds_type == 2 && base.bounds.x > 0.0 && base.bounds.y > 0.0 {
        parts.push(format!("width: {}px",  base.bounds.x));
        parts.push(format!("height: {}px", base.bounds.y));
    }

    parts.join("; ")
}

fn resolve_asset(file: &str, asset_map: &HashMap<String, Option<String>>) -> String {
    if let Some(Some(web_path)) = asset_map.get(file) {
        return web_path.clone();
    }
    // Fallback: original path with forward slashes (useful for local testing)
    file.replace('\\', "/")
}

fn build_css_filters(filters: &[Filter]) -> String {
    let mut parts = Vec::new();
    for f in filters {
        let s = &f.settings;
        if f.id == "color_filter" || f.id == "color_filter_v2" {
            if let Some(v) = s["contrast"].as_f64()   { parts.push(format!("contrast({:.3})",   1.0 + v)); }
            if let Some(v) = s["brightness"].as_f64() { parts.push(format!("brightness({:.3})", 1.0 + v)); }
            if let Some(v) = s["saturation"].as_f64() { parts.push(format!("saturate({:.3})",   1.0 + v)); }
            if let Some(v) = s["gamma"].as_f64()      { parts.push(format!("brightness({:.3})", 1.0 + v * 0.5)); }
        }
        if f.id == "obs_composite_blur" {
            parts.push("blur(4px)".to_string());
        }
    }
    parts.join(" ")
}
