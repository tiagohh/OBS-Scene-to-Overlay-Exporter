use tauri::Emitter;

mod assets;
mod generator;
mod parser;
mod utils;

// ─── Return types ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ExportResult {
    pub success: bool,
    pub copied:  usize,
    pub missing: usize,
    pub message: String,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// List all scene names in the given OBS scene collection JSON.
#[tauri::command]
fn list_scenes(json_content: String) -> Result<Vec<String>, String> {
    let json: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("JSON inválido: {}", e))?;

    let empty = vec![];
    let scenes: Vec<String> = json["sources"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|s| s["id"].as_str() == Some("scene"))
        .map(|s| utils::fix_mojibake(s["name"].as_str().unwrap_or("")))
        .collect();

    if scenes.is_empty() {
        return Err("Nenhuma cena encontrada neste arquivo.".into());
    }

    Ok(scenes)
}

/// Parse and export one scene to an HTML overlay folder.
#[tauri::command]
async fn export_overlay(
    window:       tauri::WebviewWindow,
    json_content: String,
    scene_name:   String,
    output_dir:   String,
) -> Result<ExportResult, String> {
    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("JSON inválido: {}", e))?;

    // Parse scene
    let scene_data = parser::parse_scene(&json, &scene_name)
        .map_err(|e| e.to_string())?;

    let _ = window.emit("progress", format!("✓ {} itens visíveis encontrados", scene_data.items.len()));

    // Create output directory
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Não foi possível criar a pasta de output: {}", e))?;

    // Process assets (copy files + download fonts)
    let win = window.clone();
    let asset_result = assets::process_assets(&scene_data, &output_dir, move |msg: String| {
        let _ = win.emit("progress", msg);
    })
    .await;

    // Generate HTML
    let html = generator::generate_html(&scene_data, &asset_result.asset_map, &asset_result.font_css);
    let html_path = std::path::Path::new(&output_dir).join("index.html");
    std::fs::write(&html_path, html.as_bytes())
        .map_err(|e| format!("Não foi possível escrever index.html: {}", e))?;

    let _ = window.emit("progress", "✓ index.html gerado".to_string());

    let copied  = asset_result.asset_map.values().filter(|v| v.is_some()).count();
    let missing = asset_result.asset_map.values().filter(|v| v.is_none()).count();

    Ok(ExportResult {
        success: true,
        copied,
        missing,
        message: format!(
            "Pronto! {} asset(s) copiado(s){}.",
            copied,
            if missing > 0 { format!(", {} não encontrado(s)", missing) } else { String::new() }
        ),
    })
}

// ─── App entry ────────────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_scenes, export_overlay])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
