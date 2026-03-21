> ⚠️ **UNDER DEVELOPMENT** — This tool is actively being developed and may produce incomplete or inaccurate overlays. Use at your own risk.

---

# OBS Scene to Overlay Exporter

A desktop app that converts your OBS Studio scene collection into a self-contained HTML overlay — ready to host on any static server and use as a browser source in cloud OBS services.

## What it does

1. Reads your OBS scene collection `.json` file
2. Lets you pick which scene to export
3. Copies all local assets (videos, images, GIFs) to an `assets/` folder
4. Converts chroma key videos to WebM VP9 with alpha channel (requires ffmpeg — see below)
5. Downloads Google Font equivalents for Windows system fonts (self-hosted WOFF2)
6. Generates an `index.html` that replicates your OBS scene layout

The output folder can be uploaded to **Netlify, Vercel, GitHub Pages**, or any static host — then used as a browser source URL in cloud OBS services.

## Supported source types

| OBS Source | HTML Output |
|---|---|
| Image | `<img>` |
| Video (mp4/webm) | `<canvas>` with JS chroma key |
| GIF | `<img>` |
| Text (GDI+ / FreeType 2) | `<div>` with font, color, outline |
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
5. Click **Export Overlay**
6. Upload the generated `cenas/{scene-name}/` folder to your static host
7. Use the hosted URL as a Browser Source in your cloud OBS

## ffmpeg (optional, recommended)

ffmpeg enables automatic conversion of chroma key videos to WebM VP9 with alpha channel, which produces the cleanest result.

**Without ffmpeg:** chroma key videos are copied as the original MP4 file. The built-in canvas JS still removes the green background when the page is served over HTTP — it works, but the result may have slight residual color fringing compared to a pre-keyed WebM. For most use cases this is perfectly acceptable.

**Install steps (Windows):**
1. Download **ffmpeg essentials build** from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) → `ffmpeg-release-essentials.zip`
2. Extract to `C:\ffmpeg\`
3. Add to PATH: open **System Properties → Advanced → Environment Variables**, edit **Path** under System Variables, add `C:\ffmpeg\<build-name>\bin`
4. Verify: open a new terminal and run `ffmpeg -version`

The app detects ffmpeg automatically at export time and logs whether conversion ran or fell back to the original file.

## Building from source

```bash
cargo build --release
# output: target/release/obs-overlay-exporter.exe
```

**Requirements:** Rust toolchain (https://rustup.rs)

## Known limitations

- **Chroma key requires HTTP** — the canvas JS chroma key is blocked by browser security when opening the HTML via `file://`. Serve via HTTP (e.g. `python -m http.server 8765`) or upload to a static host for it to work correctly.
- **Text rendering is close but not pixel-perfect** — OBS renders text using the FreeType 2 engine; browsers on Windows use DirectWrite. Even with the exact same font file, the two engines produce slightly different character spacing, glyph weight, and anti-aliasing. Colors, size, outline, and drop shadow are matched as closely as CSS allows, but a pixel-perfect match is not achievable.
- **Windows system fonts** — fonts such as OCR A Extended are used directly if available on the machine opening the HTML. Google Fonts equivalents are downloaded as fallback for remote servers. The fallback font may look noticeably different from the original.
- **Browser sources** (Streamlabs Chat Box, StreamElements alerts, etc.) are embedded as `<iframe>` and depend entirely on those external services being online and allowing embedding. Loading errors or visual glitches from these widgets are caused by the external service, not by this tool.
- **Some OBS filter effects** are not yet supported.

## License

MIT
