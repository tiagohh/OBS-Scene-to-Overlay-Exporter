> ⚠️ **UNDER DEVELOPMENT** — This tool is actively being developed and may produce incomplete or inaccurate overlays. Use at your own risk.

---

# OBS Scene to Overlay Exporter

A desktop app that converts your OBS Studio scene collection into a self-contained HTML overlay — ready to host on any static server and use as a browser source in cloud OBS services (e.g. Antiscuff).

## What it does

1. Reads your OBS scene collection `.json` file
2. Lets you pick which scene to export
3. Copies all local assets (videos, images, GIFs) to an `assets/` folder
4. Downloads Google Font equivalents for Windows system fonts (self-hosted WOFF2)
5. Generates a pixel-accurate `index.html` that replicates your OBS scene layout

The output folder can be uploaded to **Netlify, Vercel, GitHub Pages**, or any static host — then used as a browser source URL in cloud OBS services.

## Supported source types

| OBS Source | HTML Output |
|---|---|
| Image | `<img>` |
| Video (mp4/webm) | `<canvas>` with JS chroma key |
| GIF | `<img>` |
| Text (GDI+) | `<div>` with font, color, outline |
| Browser Source | `<iframe>` |
| Color Source | `<div>` with background color |
| Group | Recursive `<div>` |
| Nested Scene | Recursive `<div>` |
| Audio | Skipped (no visual) |

## How to use

1. In OBS Studio: **Scene Collection → Export**
2. Open `obs-overlay-exporter.exe`
3. Select or drag the exported `.json` file
4. Pick the scene from the list
5. Click **Exportar Overlay**
6. Upload the generated `cenas/{scene-name}/` folder to your static host
7. Use the hosted URL as a Browser Source in your cloud OBS

## Building from source

```bash
cd src
cargo build --release
# output: src/target/release/obs-overlay-exporter.exe
```

**Requirements:** Rust toolchain (https://rustup.rs)

## Known limitations

- Chroma key on videos works via canvas JS — requires serving over HTTP (not `file://`) for cross-origin video sources
- Windows system fonts (e.g. OCR A Extended) are substituted with Google Fonts equivalents when not available on the host machine
- Some complex OBS filter effects are not yet supported
- Groups are a known quirk in OBS's JSON format and may behave unexpectedly in edge cases

## License

MIT
