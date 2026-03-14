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
    pub visible:     bool,
    pub pos:         Pos,
    pub scale:       Scale,
    pub rot:         f64,
    pub bounds_type: i64,
    pub bounds:      Bounds,
    pub filters:     Vec<Filter>,
    /// OBS item-level alignment: bitmask — left=1, right=2, top=4, bottom=8.
    /// 5 = top-left (default). Determines what point of the item pos.x/y refers to.
    pub item_align:  i64,
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

pub fn parse_scene(json: &Value, scene_name: &str, only_visible: bool) -> Result<SceneData, String> {
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

    // OBS has a known bug: group children appear BOTH in the scene's flat items list
    // AND inside the group source's settings.items. We must filter them from the flat
    // list so they are only rendered once (as part of the group).
    // Strategy: find every group in this scene's items → collect their children's
    // source_uuids → exclude those from the top-level parse pass.
    let mut group_child_uuids: HashSet<String> = HashSet::new();
    for item in &raw_items {
        let uuid = item["source_uuid"].as_str().unwrap_or("");
        let name = item["name"].as_str().unwrap_or("");
        let src  = by_uuid.get(uuid).or_else(|| by_name.get(name));
        if let Some(src) = src {
            if src["id"].as_str() == Some("group") {
                if let Some(children) = src["settings"]["items"].as_array() {
                    for child in children {
                        if let Some(child_uuid) = child["source_uuid"].as_str() {
                            group_child_uuids.insert(child_uuid.to_string());
                        }
                    }
                }
            }
        }
    }
    let top_level_items: Vec<Value> = raw_items
        .into_iter()
        .filter(|item| {
            let uuid = item["source_uuid"].as_str().unwrap_or("");
            !group_child_uuids.contains(uuid)
        })
        .collect();

    let items = parse_items(&top_level_items, &by_uuid, &by_name, &canvas, only_visible);
    let fonts = collect_fonts(&items);

    Ok(SceneData { name: scene_name.to_string(), canvas, items, fonts })
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn parse_items(
    raw:          &[Value],
    by_uuid:      &HashMap<String, Value>,
    by_name:      &HashMap<String, Value>,
    canvas:       &Canvas,
    only_visible: bool,
) -> Vec<SceneItem> {
    let mut result = Vec::new();
    for item in raw {
        // OBS saves visibility as bool false OR integer 0
        let invisible = item["visible"].as_bool() == Some(false)
            || item["visible"].as_i64() == Some(0);
        if only_visible && invisible {
            continue;
        }

        let source_uuid = item["source_uuid"].as_str().unwrap_or("");
        let item_name   = item["name"].as_str().unwrap_or("");

        let source = by_uuid
            .get(source_uuid)
            .or_else(|| by_name.get(item_name));

        if let Some(source) = source {
            if let Some(parsed) = parse_item(item, source, by_uuid, by_name, canvas, only_visible) {
                result.push(parsed);
            }
        }
    }
    result
}

fn parse_item(
    item:         &Value,
    source:       &Value,
    by_uuid:      &HashMap<String, Value>,
    by_name:      &HashMap<String, Value>,
    canvas:       &Canvas,
    only_visible: bool,
) -> Option<SceneItem> {
    let is_visible = item["visible"].as_bool().unwrap_or(true)
        && item["visible"].as_i64().unwrap_or(1) != 0;

    let base = BaseItem {
        id:      format!("item-{}", item["id"].as_i64().unwrap_or(0)),
        name:    fix_mojibake(item["name"].as_str().unwrap_or("")),
        visible: is_visible,
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
        item_align:  item["align"].as_i64().unwrap_or(5),
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
            let items = parse_items(&raw, by_uuid, by_name, canvas, only_visible);
            Some(SceneItem::Group { base, items })
        }

        "scene" => {
            // source is already the nested scene; just parse its items
            if let Some(arr) = source["settings"]["items"].as_array() {
                let nested = parse_items(arr, by_uuid, by_name, canvas, only_visible);
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
