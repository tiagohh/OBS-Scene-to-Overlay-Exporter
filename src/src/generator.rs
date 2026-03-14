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

  <script>
  // ── Auto-scale canvas to fit viewport (local preview only) ───────────────────
  // When used as OBS browser source, the viewport is exactly {cw}x{ch}, so
  // scale will be 1.0 and nothing changes. Smaller windows scale down to fit.
  (function() {{
    var CW={cw}, CH={ch};
    function fit() {{
      var s = Math.min(window.innerWidth/CW, window.innerHeight/CH);
      if (s < 0.999) {{
        document.documentElement.style.transformOrigin = 'top left';
        document.documentElement.style.transform = 'scale(' + s + ')';
        document.documentElement.style.width  = CW + 'px';
        document.documentElement.style.height = CH + 'px';
        document.body.style.overflow = 'hidden';
      }} else {{
        document.documentElement.style.transform = '';
      }}
    }}
    fit();
    window.addEventListener('resize', fit);
  }})();

  // ── OBS Overlay Diagnostic — open F12 → Console to see this report ──────────
  window.addEventListener('load', function() {{
    var CW = {cw}, CH = {ch};
    console.log('%c=== OBS Overlay Diagnostic ===', 'font-weight:bold;font-size:14px;color:#0af');
    console.log('Canvas: ' + CW + 'x' + CH);
    console.log('Browser viewport: ' + window.innerWidth + 'x' + window.innerHeight);
    if (window.innerWidth < CW || window.innerHeight < CH) {{
      console.warn('⚠ Viewport smaller than canvas! Zoom out (Ctrl+-) until you see the full ' + CW + 'x' + CH + ' canvas.');
    }}
    var layers = document.querySelectorAll('.layer');
    console.log('Total layers: ' + layers.length);
    console.log('');
    layers.forEach(function(el) {{
      var r = el.getBoundingClientRect();
      var style = el.getAttribute('style') || '';
      var tag = el.tagName.toLowerCase();
      var extra = '';
      if (tag === 'img')    extra = ' | natural: ' + el.naturalWidth + 'x' + el.naturalHeight;
      if (tag === 'video')  extra = ' | video: ' + el.videoWidth + 'x' + el.videoHeight;
      if (tag === 'canvas') extra = ' | canvas attr: ' + el.width + 'x' + el.height;
      if (tag === 'div')    extra = ' | bg: ' + window.getComputedStyle(el).backgroundColor + ' | size: ' + Math.round(r.width) + 'x' + Math.round(r.height);
      var inCanvas = r.left < CW && r.top < CH && r.right > 0 && r.bottom > 0;
      var vis = inCanvas ? '✓' : '⚠ OUT-OF-BOUNDS';
      console.log(
        '%c[' + tag + '] ' + el.id + ' ' + vis,
        inCanvas ? 'color:#4f4' : 'color:#fa0',
        '| rect: (' + Math.round(r.left) + ',' + Math.round(r.top) + ') ' + Math.round(r.width) + 'x' + Math.round(r.height) + extra
      );
    }});
    console.log('');
    console.log('%c=== End of Report ===', 'font-weight:bold;color:#0af');
  }});
  </script>
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
            let style      = build_transform_style(base);
            let src        = resolve_asset(file, asset_map);
            let css_filter = build_css_filters(&base.filters);
            let filter_str = if css_filter.is_empty() { String::new() } else { format!("; filter: {}", css_filter) };
            // Color/chroma key on image → screen blend removes dark background
            let blend_str  = if get_chroma_key(&base.filters).is_some()
                || base.filters.iter().any(|f| matches!(f.id.as_str(),
                    "color_key_filter" | "color_key_filter_v2" |
                    "chroma_key_filter" | "chroma_key_filter_v2"))
            { "; mix-blend-mode: screen" } else { "" };
            Some(format!(
                "<!-- {} (image) -->\n  <img id=\"{}\" class=\"layer\" src=\"{}\" alt=\"\" style=\"{}{}{}\">",
                base.name, base.id, src, style, filter_str, blend_str
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
            let style      = build_transform_style(base);
            let src        = resolve_asset(file, asset_map);
            let filters    = build_css_filters(&base.filters);
            let filter_str = if filters.is_empty() { String::new() } else { format!("; filter: {}", filters) };
            let ext        = file.rsplit('.').next().unwrap_or("").to_lowercase();
            let mime       = if ext == "webm" { "video/webm" } else { "video/mp4" };

            // Chroma/color key → canvas-based pixel removal (works for any key color)
            if let Some((kr, kg, kb, tol_sq)) = get_chroma_key(&base.filters) {
                let loop_js = if *looping { "true" } else { "false" };
                return Some(format!(
                    concat!(
                        "<!-- {name} (video + chroma key) -->\n",
                        // Canvas hidden until video metadata loads (avoids flash of green)
                        "  <canvas id=\"{id}\" class=\"layer\" style=\"{style};display:none\"></canvas>\n",
                        "  <script>(function(){{\n",
                        "    var c=document.getElementById('{id}'),ctx=c.getContext('2d');\n",
                        "    var v=document.createElement('video');\n",
                        "    v.src='{src}';v.loop={loop};v.muted=true;v.playsInline=true;\n",
                        "    var kr={kr},kg={kg},kb={kb},tSq={tol_sq},started=false;\n",
                        "    function draw(){{\n",
                        "      if(c.width>0&&c.height>0){{\n",
                        "        ctx.drawImage(v,0,0);\n",
                        "        try{{\n",
                        "          var d=ctx.getImageData(0,0,c.width,c.height),p=d.data;\n",
                        "          for(var i=0;i<p.length;i+=4){{var dr=p[i]-kr,dg=p[i+1]-kg,db=p[i+2]-kb;if(dr*dr+dg*dg+db*db<tSq)p[i+3]=0;}}\n",
                        "          ctx.putImageData(d,0,0);\n",
                        "        }}catch(e){{}}\n",
                        "      }}\n",
                        "      requestAnimationFrame(draw);\n",
                        "    }}\n",
                        "    v.addEventListener('loadedmetadata',function(){{\n",
                        "      c.width=v.videoWidth;c.height=v.videoHeight;c.style.display='';\n",
                        "      if(!started){{started=true;requestAnimationFrame(draw);}}\n",
                        "    }});\n",
                        "    v.play().catch(function(){{}});\n",
                        "  }})();</script>",
                    ),
                    name    = base.name,
                    id      = base.id,
                    style   = style,
                    src     = src,
                    loop    = loop_js,
                    kr      = kr,
                    kg      = kg,
                    kb      = kb,
                    tol_sq  = tol_sq,
                ));
            }

            // Normal video (no chroma key, or black key → screen blend)
            let loop_attr  = if *looping { " loop" } else { "" };
            let black_key  = base.filters.iter().any(|f| {
                matches!(f.id.as_str(), "color_key_filter" | "color_key_filter_v2" |
                                        "chroma_key_filter" | "chroma_key_filter_v2")
            });
            let blend_str  = if black_key { "; mix-blend-mode: screen" } else { "" };
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

            // Local Windows font first — if the HTML is opened on the same machine
            // as OBS, the original font is used. Google Font is fallback for servers.
            let font_family = match get_web_font(font_face) {
                Some(m) if m.google_font.is_some() =>
                    format!("'{}', '{}', monospace", font_face, m.web),
                Some(m) => m.web.to_string(),
                None    => format!("'{}', sans-serif", font_face),
            };

            let text_color = obs_color_to_css(*color, *opacity);

            // OBS default bk_color is opaque black (0xFF000000).
            // If not present in JSON, fall back to black so opacity applies correctly.
            let effective_bk = bk_color.or(Some(0xFF000000u32 as i64));
            let bg_css = if *bk_opacity > 0.0 {
                obs_color_to_css(effective_bk, *bk_opacity)
            } else {
                "transparent".to_string()
            };

            let mut shadows: Vec<String> = Vec::new();
            if *outline && *outline_size > 0 {
                // OBS outline_size is in canvas-space pixels (post-scale).
                // Divide by item scale so the CSS shadow produces the correct
                // visual size after transform: scale() is applied.
                let scale = base.scale.x.abs().max(0.01);
                let css_outline_px = *outline_size as f64 / scale;
                shadows.push(build_outline_shadow(css_outline_px, *outline_color));
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

            // padding only when background is visible; line-height 1 matches OBS
            let padding = if *bk_opacity > 0.0 { "; padding: 2px 6px" } else { "" };

            let full_style = format!(
                "{}; font-family: {}; font-size: {}px; font-weight: {}; font-style: {}; \
                 color: {}; background: {}; text-shadow: {}; text-align: {}; \
                 line-height: 1{}{}",
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
                padding,
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

    // OBS item_align bitmask: left=1, right=2, top=4, bottom=8.
    // pos.x/y is the anchor point indicated by the alignment.
    // We use CSS translate to shift the element so that anchor point lands on pos.x/y.
    let h_align = base.item_align & 0b0011; // bits 0-1: 0=center, 1=left, 2=right
    let v_align = base.item_align & 0b1100; // bits 2-3: 0=center, 4=top, 8=bottom
    let tx = match h_align { 1 => "0%", 2 => "-100%", _ => "-50%" }; // left / right / center
    let ty = match v_align { 4 => "0%", 8 => "-100%", _ => "-50%" }; // top  / bottom / center

    let mut transforms = Vec::new();

    // Anchor offset first (before scale/rotate so it's in the item's local space)
    if tx != "0%" || ty != "0%" {
        transforms.push(format!("translate({}, {})", tx, ty));
    }

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

/// Returns (R, G, B, tolerance²) for the key color if a chroma/color key filter is present.
/// OBS similarity 1-1000 → we map to Euclidean tolerance ≈ similarity/2,
/// then store tolerance² to avoid sqrt in the JS hot loop.
fn get_chroma_key(filters: &[Filter]) -> Option<(u32, u32, u32, u32)> {
    for f in filters {
        let s = &f.settings;
        let sim = s["similarity"].as_f64().unwrap_or(80.0).clamp(1.0, 1000.0);
        let tol = (sim / 2.0) as u32;

        match f.id.as_str() {
            "chroma_key_filter" | "chroma_key_filter_v2" => {
                let (r, g, b) = match s["key_color_type"].as_str().unwrap_or("green") {
                    "blue"    => (0u32, 0u32, 255u32),
                    "magenta" => (255u32, 0u32, 255u32),
                    "custom"  => {
                        let c = s["custom_color"].as_i64().unwrap_or(0) as u32;
                        (c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF)
                    }
                    _ => (0u32, 255u32, 0u32), // green
                };
                return Some((r, g, b, tol * tol));
            }
            "color_key_filter" | "color_key_filter_v2" => {
                let c = s["color"].as_i64().unwrap_or(0) as u32;
                let r = c & 0xFF;
                let g = (c >> 8) & 0xFF;
                let b = (c >> 16) & 0xFF;
                // Black key → mix-blend-mode: screen is good enough; skip canvas
                if r < 10 && g < 10 && b < 10 {
                    return None;
                }
                return Some((r, g, b, tol * tol));
            }
            _ => {}
        }
    }
    None
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
