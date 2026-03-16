> ⚠️ **UNDER DEVELOPMENT** — This tool is actively being developed and may produce incomplete or inaccurate overlays. Use at your own risk.

---

# OBS Scene to Overlay Exporter

A desktop app that converts your OBS Studio scene collection into a self-contained HTML overlay — ready to host on any static server and use as a browser source in cloud OBS services (e.g. Antiscuff).

## What it does

1. Reads your OBS scene collection `.json` file
2. Lets you pick which scene to export
3. Copies all local assets (videos, images, GIFs) to an `assets/` folder
4. Converts chroma key videos to WebM VP9 with alpha channel (requires ffmpeg)
5. Downloads Google Font equivalents for Windows system fonts (self-hosted WOFF2)
6. Generates a pixel-accurate `index.html` that replicates your OBS scene layout

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

## ffmpeg (optional, recommended)

ffmpeg enables automatic conversion of chroma key videos to WebM VP9 with alpha channel, which produces a cleaner result.

**Install steps (Windows):**
1. Download **ffmpeg essentials build** from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) → `ffmpeg-release-essentials.zip`
2. Extract to `C:\ffmpeg\`
3. Add to PATH: open **System Properties → Advanced → Environment Variables**, edit **Path** under System Variables, add `C:\ffmpeg\<build-name>\bin`
4. Verify: open a new terminal and run `ffmpeg -version`

Without ffmpeg, videos with chroma key are copied as-is and the canvas JS still removes the green background via HTTP.

## Building from source

```bash
cd src
cargo build --release
# output: src/target/release/obs-overlay-exporter.exe
```

**Requirements:** Rust toolchain (https://rustup.rs)

## Known limitations

- Chroma key on videos requires serving over HTTP (not `file://`) — a green tint is expected when opening the HTML directly from disk
- Windows system fonts (e.g. OCR A Extended) are used directly if available on the machine opening the HTML; Google Fonts equivalents are downloaded as fallback for remote servers
- Some complex OBS filter effects are not yet supported

## License

MIT
