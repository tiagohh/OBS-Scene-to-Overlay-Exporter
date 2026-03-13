use serde_json::Value;
use std::collections::{HashMap, HashSet};
use crate::utils::fix_mojibake;

// ─── Data structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Canvas {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct Pos { pub x: f64, pub y: f64 }

#[derive(Debug, Clone)]
pub struct Scale { pub x: f64, pub y: f64 }

#[derive(Debug, Clone)]
pub struct Bounds { pub x: f64, pub y: f64 }

#[derive(Debug, Clone)]
pub struct Filter {
    pub id:       String,
    pub settings: Value,
}

#[derive(Debug, Clone)]
pub struct BaseItem {
    pub id:          String,
    pub name:        String,
    pub pos:         Pos,
    pub scale:       Scale,
    pub rot:         f64,
    pub bounds_type: i64,
    pub bounds:      Bounds,
    pub filters:     Vec<Filter>,
}

#[derive(Debug, Clone)]
pub enum SceneItem {
    Image   { base: BaseItem, file: String },
    Video   { base: BaseItem, file: String, looping: bool, speed_percent: f64 },
    Gif     { base: BaseItem, file: String },
    Audio   { base: BaseItem, file: String },
    Text    {
        base:          BaseItem,
        text:          String,
        font_face:     String,
        font_size:     f64,
        font_bold:     bool,
        font_italic:   bool,
        color:         Option<i64>,
        opacity:       f64,
        bk_color:      Option<i64>,
        bk_opacity:    f64,
        outline:       bool,
        outline_size:  i64,
        outline_color: Option<i64>,
        text_align:    String,
        drop_shadow:   bool,
        custom_width:  i64,
        vertical:      bool,
    },
    Browser { base: BaseItem, url: String, width: f64, height: f64 },
    Color   { base: BaseItem, color: Option<i64>, width: f64, height: f64 },
    Group   { base: BaseItem, items: Vec<SceneItem> },
}

impl SceneItem {
    /// Returns the local file path for media items, None otherwise.
    pub fn file(&self) -> Option<&str> {
        match self {
            SceneItem::Image { file, .. } => Some(file),
            SceneItem::Video { file, .. } => Some(file),
            SceneItem::Gif   { file, .. } => Some(file),
            SceneItem::Audio { file, .. } => Some(file),
            _ => None,
        }
    }
}

pub struct SceneData {
    pub name:   String,
    pub canvas: Canvas,
    pub items:  Vec<SceneItem>,
    pub fonts:  HashSet<String>,
}

// ─── Public API ───────────────────────────────────────────────────────────────

pub fn parse_scene(json: &Value, scene_name: &str) -> Result<SceneData, String> {
    let canvas = Canvas {
        x: json["resolution"]["x"].as_f64().unwrap_or(1920.0),
        y: json["resolution"]["y"].as_f64().unwrap_or(1080.0),
    };

    let empty = vec![];
    let sources = json["sources"].as_array().unwrap_or(&empty);

    // Build lookup indices (clone Values to avoid lifetime issues)
    let mut by_uuid: HashMap<String, Value> = HashMap::new();
    let mut by_name: HashMap<String, Value> = HashMap::new();

    for src in sources {
        if let Some(uuid) = src["uuid"].as_str() {
            by_uuid.insert(uuid.to_string(), src.clone());
        }
        if let Some(name) = src["name"].as_str() {
            by_name.insert(name.to_string(), src.clone());
        }
    }

    // Also index group sources from json.groups
    let empty2 = vec![];
    for grp in json["groups"].as_array().unwrap_or(&empty2) {
        if let Some(uuid) = grp["uuid"].as_str() {
            by_uuid.insert(uuid.to_string(), grp.clone());
        }
        if let Some(name) = grp["name"].as_str() {
            by_name.insert(name.to_string(), grp.clone());
        }
    }

    // Find the requested scene (try exact name, then mojibake-fixed name)
    let scene_source = sources
        .iter()
        .find(|s| {
            s["id"].as_str() == Some("scene") && {
                let name = s["name"].as_str().unwrap_or("");
                name == scene_name || fix_mojibake(name) == scene_name
            }
        })
        .ok_or_else(|| {
            let available: Vec<String> = sources
                .iter()
                .filter(|s| s["id"].as_str() == Some("scene"))
                .map(|s| format!("  • {}", fix_mojibake(s["name"].as_str().unwrap_or(""))))
                .collect();
            format!(
                "Cena \"{}\" não encontrada.\nCenas disponíveis:\n{}",
                scene_name,
                available.join("\n")
            )
        })?;

    let raw_items = scene_source["settings"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let items = parse_items(&raw_items, &by_uuid, &by_name, &canvas);
    let fonts = collect_fonts(&items);

    Ok(SceneData { name: scene_name.to_string(), canvas, items, fonts })
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn parse_items(
    raw:     &[Value],
    by_uuid: &HashMap<String, Value>,
    by_name: &HashMap<String, Value>,
    canvas:  &Canvas,
) -> Vec<SceneItem> {
    let mut result = Vec::new();
    for item in raw {
        // Skip invisible items
        if item["visible"].as_bool() == Some(false) {
            continue;
        }

        let source_uuid = item["source_uuid"].as_str().unwrap_or("");
        let item_name   = item["name"].as_str().unwrap_or("");

        let source = by_uuid
            .get(source_uuid)
            .or_else(|| by_name.get(item_name));

        if let Some(source) = source {
            if let Some(parsed) = parse_item(item, source, by_uuid, by_name, canvas) {
                result.push(parsed);
            }
        }
    }
    result
}

fn parse_item(
    item:    &Value,
    source:  &Value,
    by_uuid: &HashMap<String, Value>,
    by_name: &HashMap<String, Value>,
    canvas:  &Canvas,
) -> Option<SceneItem> {
    let base = BaseItem {
        id:   format!("item-{}", item["id"].as_i64().unwrap_or(0)),
        name: fix_mojibake(item["name"].as_str().unwrap_or("")),
        pos:  Pos {
            x: item["pos"]["x"].as_f64().unwrap_or(0.0),
            y: item["pos"]["y"].as_f64().unwrap_or(0.0),
        },
        scale: Scale {
            x: item["scale"]["x"].as_f64().unwrap_or(1.0),
            y: item["scale"]["y"].as_f64().unwrap_or(1.0),
        },
        rot:         item["rot"].as_f64().unwrap_or(0.0),
        bounds_type: item["bounds_type"].as_i64().unwrap_or(0),
        bounds: Bounds {
            x: item["bounds"]["x"].as_f64().unwrap_or(0.0),
            y: item["bounds"]["y"].as_f64().unwrap_or(0.0),
        },
        filters: source["filters"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|f| Filter {
                id:       f["id"].as_str().unwrap_or("").to_string(),
                settings: f["settings"].clone(),
            })
            .collect(),
    };

    let source_id = source["id"].as_str().unwrap_or("");
    let s         = &source["settings"];

    match source_id {
        "image_source" => {
            let file = s["file"].as_str().unwrap_or("").to_string();
            Some(SceneItem::Image { base, file })
        }

        "ffmpeg_source" => {
            let file = s["local_file"]
                .as_str()
                .or_else(|| s["input"].as_str())
                .unwrap_or("")
                .to_string();
            let ext = file.rsplit('.').next().unwrap_or("").to_lowercase();

            match ext.as_str() {
                "gif" => Some(SceneItem::Gif { base, file }),
                "mp3" | "wav" | "ogg" | "flac" | "aac" => Some(SceneItem::Audio { base, file }),
                _ => Some(SceneItem::Video {
                    looping:       s["looping"].as_bool().unwrap_or(true),
                    speed_percent: s["speed_percent"].as_f64().unwrap_or(100.0),
                    base,
                    file,
                }),
            }
        }

        "text_gdiplus" | "text_gdiplus_v2" | "text_gdiplus_v3" | "text_ft2_source" => {
            let flags = s["font"]["flags"].as_i64().unwrap_or(0);
            Some(SceneItem::Text {
                text:          fix_mojibake(s["text"].as_str().unwrap_or("")),
                font_face:     s["font"]["face"].as_str().unwrap_or("Arial").to_string(),
                font_size:     s["font"]["size"].as_f64().unwrap_or(30.0),
                font_bold:     (flags & 1) != 0,
                font_italic:   (flags & 2) != 0,
                color:         s["color"].as_i64(),
                opacity:       s["opacity"].as_f64().unwrap_or(100.0),
                bk_color:      s["bk_color"].as_i64(),
                bk_opacity:    s["bk_opacity"].as_f64().unwrap_or(0.0),
                outline:       s["outline"].as_bool().unwrap_or(false),
                outline_size:  s["outline_size"].as_i64().unwrap_or(3),
                outline_color: s["outline_color"].as_i64(),
                text_align:    s["align"].as_str().unwrap_or("left").to_string(),
                drop_shadow:   s["drop_shadow"].as_bool().unwrap_or(false),
                custom_width:  s["custom_width"].as_i64().unwrap_or(0),
                vertical:      s["vertical"].as_bool().unwrap_or(false),
                base,
            })
        }

        "color_source" | "color_source_v3" => Some(SceneItem::Color {
            color:  s["color"].as_i64(),
            width:  s["width"].as_f64().unwrap_or(canvas.x),
            height: s["height"].as_f64().unwrap_or(canvas.y),
            base,
        }),

        "browser_source" => Some(SceneItem::Browser {
            url:    s["url"].as_str().unwrap_or("").trim().to_string(),
            width:  s["width"].as_f64().unwrap_or(canvas.x),
            height: s["height"].as_f64().unwrap_or(canvas.y),
            base,
        }),

        "group" => {
            let raw = s["items"].as_array().cloned().unwrap_or_default();
            let items = parse_items(&raw, by_uuid, by_name, canvas);
            Some(SceneItem::Group { base, items })
        }

        "scene" => {
            // source is already the nested scene; just parse its items
            if let Some(arr) = source["settings"]["items"].as_array() {
                let nested = parse_items(arr, by_uuid, by_name, canvas);
                return Some(SceneItem::Group { base, items: nested });
            }
            None
        }

        // Audio-only or unknown — skip
        _ => None,
    }
}

fn collect_fonts(items: &[SceneItem]) -> HashSet<String> {
    let mut fonts = HashSet::new();
    for item in items {
        if let SceneItem::Text { font_face, .. } = item {
            fonts.insert(font_face.clone());
        }
        if let SceneItem::Group { items: children, .. } = item {
            for f in collect_fonts(children) {
                fonts.insert(f);
            }
        }
    }
    fonts
}
