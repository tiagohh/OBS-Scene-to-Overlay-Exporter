use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;

// ─── Messages from export thread → UI ─────────────────────────────────────────

enum Msg {
    Log(String),
    Done { output_dir: PathBuf },
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct App {
    // Step 1 — JSON file
    json_path:    String,
    json_content: Option<String>,

    // Step 2 — Scene list
    scenes:   Vec<String>,
    selected: Option<usize>,

    // Option
    only_visible: bool,

    // Log + status
    log:          Vec<String>,
    exporting:    bool,
    output_path:  Option<PathBuf>,

    // Channel: export thread → UI
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Msg>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            json_path:    String::new(),
            json_content: None,
            scenes:       Vec::new(),
            selected:     None,
            only_visible: true,
            log:          Vec::new(),
            exporting:    false,
            output_path:  None,
            tx,
            rx,
        }
    }
}

// ─── egui App impl ────────────────────────────────────────────────────────────

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain messages from the export thread
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Log(s) => self.log.push(s),
                Msg::Done { output_dir } => {
                    self.exporting   = false;
                    self.output_path = Some(output_dir);
                }
            }
        }
        // Keep repainting while export is running
        if self.exporting {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // Handle JSON file drop
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.iter().find_map(|f| {
                f.path.clone().filter(|p: &PathBuf| {
                    p.extension().map(|e| e == "json").unwrap_or(false)
                })
            })
        });
        if let Some(path) = dropped {
            self.json_path = path.to_string_lossy().to_string();
            self.load_json();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OBS Overlay Exporter");
            ui.add_space(10.0);

            // ── 1. Arquivo ───────────────────────────────────────────────────
            ui.label("1. Arquivo de cenas exportado do OBS:");
            ui.horizontal(|ui| {
                let w = ui.available_width() - 75.0;
                ui.add(egui::TextEdit::singleline(&mut self.json_path).desired_width(w));
                if ui.button("Abrir…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("OBS Scene Collection", &["json"])
                        .pick_file()
                    {
                        self.json_path = p.to_string_lossy().to_string();
                        self.load_json();
                    }
                }
            });
            ui.small("Ou arraste o arquivo .json para a janela");
            ui.add_space(14.0);

            // ── 2. Lista de cenas ────────────────────────────────────────────
            if !self.scenes.is_empty() {
                ui.label("2. Selecionar cena:");
                ui.group(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("scenes")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            let mut clicked_scene: Option<usize> = None;
                            for (i, scene) in self.scenes.iter().enumerate() {
                                let selected = self.selected == Some(i);
                                if ui.selectable_label(selected, scene).clicked() {
                                    clicked_scene = Some(i);
                                }
                            }
                            if let Some(i) = clicked_scene {
                                self.selected = Some(i);
                                self.preview_scene(i);
                            }
                        });
                });
                ui.add_space(10.0);

                // ── Checkbox ─────────────────────────────────────────────────
                ui.checkbox(
                    &mut self.only_visible,
                    "Apenas itens visíveis no OBS (recomendado)",
                );
                ui.add_space(12.0);

                // ── Exportar ─────────────────────────────────────────────────
                let can_export = self.selected.is_some() && !self.exporting;
                if ui
                    .add_enabled(
                        can_export,
                        egui::Button::new(
                            egui::RichText::new("  Exportar Overlay  ").size(16.0),
                        ),
                    )
                    .clicked()
                {
                    self.start_export();
                }
            }

            // ── Log ──────────────────────────────────────────────────────────
            if !self.log.is_empty() {
                ui.add_space(12.0);
                ui.separator();
                ui.label("Log:");
                egui::ScrollArea::vertical()
                    .id_salt("log")
                    .stick_to_bottom(true)
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.label(line);
                        }
                    });
            }

            // ── Output path ──────────────────────────────────────────────────
            if let Some(path) = &self.output_path {
                ui.add_space(6.0);
                ui.colored_label(
                    egui::Color32::from_rgb(100, 220, 100),
                    format!("✓ Output: {}", path.display()),
                );
            }
        });
    }
}

// ─── Logic ────────────────────────────────────────────────────────────────────

impl App {
    fn load_json(&mut self) {
        self.scenes.clear();
        self.selected     = None;
        self.log.clear();
        self.output_path  = None;
        self.json_content = None;

        match std::fs::read_to_string(&self.json_path) {
            Err(e) => {
                self.log.push(format!("✗ Não foi possível abrir o arquivo: {}", e));
            }
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Err(e) => {
                    self.log.push(format!("✗ JSON inválido: {}", e));
                }
                Ok(json) => {
                    let scenes: Vec<String> = json["sources"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter(|s| s["id"].as_str() == Some("scene"))
                        .map(|s| crate::utils::fix_mojibake(s["name"].as_str().unwrap_or("")))
                        .collect();

                    if scenes.is_empty() {
                        self.log.push("⚠ Nenhuma cena encontrada no arquivo.".to_string());
                    } else {
                        self.json_content = Some(content);
                        self.scenes       = scenes;
                    }
                }
            },
        }
    }

    fn preview_scene(&mut self, scene_idx: usize) {
        let scene_name   = self.scenes[scene_idx].clone();
        let json_content = match &self.json_content { Some(c) => c.clone(), None => return };

        self.log.clear();
        self.output_path = None;

        let json: serde_json::Value = match serde_json::from_str(&json_content) {
            Ok(j)  => j,
            Err(e) => { self.log.push(format!("✗ JSON inválido: {}", e)); return; }
        };

        // Parse ALL items (only_visible=false) so we see hidden ones too
        let scene_data = match crate::parser::parse_scene(&json, &scene_name, false) {
            Ok(s)  => s,
            Err(e) => { self.log.push(format!("✗ {}", e)); return; }
        };

        self.log.push(format!("── Cena: {} ──────────────────────────", scene_name));
        self.log.push(format!("   Canvas: {}×{}", scene_data.canvas.x, scene_data.canvas.y));
        self.log.push(String::new());
        self.log_items(&scene_data.items, 0);

        // Auto-save log to debug.log next to the exe's parent (project root)
        let log_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("debug.log");
        let _ = std::fs::write(&log_path, self.log.join("\n"));
    }

    fn log_items(&mut self, items: &[crate::parser::SceneItem], depth: usize) {
        use crate::parser::SceneItem;
        let indent = "  ".repeat(depth);
        for item in items {
            let base = match item {
                SceneItem::Image  { base, .. } | SceneItem::Video { base, .. } |
                SceneItem::Gif    { base, .. } | SceneItem::Audio { base, .. } |
                SceneItem::Text   { base, .. } | SceneItem::Browser{ base, .. } |
                SceneItem::Color  { base, .. } | SceneItem::Group  { base, .. } => base,
            };

            let vis_tag = if base.visible { "VIS" } else { "HID" };
            let pos_info = format!(
                "pos({:.0},{:.0}) scale({:.2},{:.2}) align={}",
                base.pos.x, base.pos.y,
                base.scale.x, base.scale.y,
                base.item_align,
            );

            let detail = match item {
                SceneItem::Image  { file, .. }  => format!("image   │ {}", short_path(file)),
                SceneItem::Video  { file, .. }  => format!("video   │ {}", short_path(file)),
                SceneItem::Gif    { file, .. }  => format!("gif     │ {}", short_path(file)),
                SceneItem::Audio  { file, .. }  => format!("audio   │ {}", short_path(file)),
                SceneItem::Text   { text, font_face, font_size, .. } => format!(
                    "text    │ font:{} {}px  \"{}\"",
                    font_face, font_size,
                    &text[..text.len().min(25)]
                ),
                SceneItem::Browser{ url, .. }   => format!("browser │ {}", url),
                SceneItem::Color  { width, height, .. } => format!("color   │ {}×{}", width, height),
                SceneItem::Group  { items: children, .. } => {
                    self.log.push(format!("[{}] {}{}  grupo ({} filhos)  {}", vis_tag, indent, base.name, children.len(), pos_info));
                    let children_clone: Vec<_> = children.clone();
                    self.log_items(&children_clone, depth + 1);
                    continue;
                }
            };

            self.log.push(format!("[{}] {}{}  {}  {}", vis_tag, indent, base.name, detail, pos_info));
        }
    }

    fn start_export(&mut self) {
        let i = match self.selected {
            Some(i) => i,
            None    => return,
        };
        let scene_name   = self.scenes[i].clone();
        let json_content = match &self.json_content {
            Some(c) => c.clone(),
            None    => return,
        };
        let only_visible = self.only_visible;

        // Output folder = {project root}/cenas/{scene name}
        // exe lives in releases\ so go up one level to reach project root
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let project_root = exe_dir.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(exe_dir);
        let output_dir = project_root.join("cenas").join(&scene_name);

        self.log.clear();
        self.log.push(format!("→ Exportando cena: {}", scene_name));
        self.log.push(format!("  Destino: {}", output_dir.display()));
        self.exporting   = true;
        self.output_path = None;

        let tx  = self.tx.clone();
        let out = output_dir.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                let send_log = |msg: String| {
                    let _ = tx.send(Msg::Log(msg));
                };

                // Parse JSON
                let json: serde_json::Value = match serde_json::from_str(&json_content) {
                    Ok(j)  => j,
                    Err(e) => {
                        send_log(format!("✗ JSON inválido: {}", e));
                        let _ = tx.send(Msg::Done { output_dir: out });
                        return;
                    }
                };

                // Parse scene
                let scene_data = match crate::parser::parse_scene(&json, &scene_name, only_visible) {
                    Ok(s)  => s,
                    Err(e) => {
                        send_log(format!("✗ {}", e));
                        let _ = tx.send(Msg::Done { output_dir: out });
                        return;
                    }
                };

                send_log(format!("✓ {} itens encontrados", scene_data.items.len()));

                // Create output dir
                if let Err(e) = std::fs::create_dir_all(&out) {
                    send_log(format!("✗ Erro ao criar pasta: {}", e));
                    let _ = tx.send(Msg::Done { output_dir: out });
                    return;
                }

                // Process assets (copy files + download fonts)
                let out_str = out.to_string_lossy().to_string();
                let tx2     = tx.clone();
                let asset_result = crate::assets::process_assets(
                    &scene_data,
                    &out_str,
                    move |msg| { let _ = tx2.send(Msg::Log(msg)); },
                )
                .await;

                // Generate HTML
                let html = crate::generator::generate_html(
                    &scene_data,
                    &asset_result.asset_map,
                    &asset_result.font_css,
                );
                match std::fs::write(out.join("index.html"), html.as_bytes()) {
                    Ok(_)  => send_log("✓ index.html gerado".to_string()),
                    Err(e) => send_log(format!("✗ Erro ao escrever index.html: {}", e)),
                }

                let copied  = asset_result.asset_map.values().filter(|v| v.is_some()).count();
                let missing = asset_result.asset_map.values().filter(|v| v.is_none()).count();
                send_log(format!(
                    "✓ Pronto! {} asset(s) copiado(s){}.",
                    copied,
                    if missing > 0 {
                        format!(", {} não encontrado(s)", missing)
                    } else {
                        String::new()
                    }
                ));

                let _ = tx.send(Msg::Done { output_dir: out });
            });
        });
    }
}

fn short_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string()
}
