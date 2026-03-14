> ⚠️ **EM DESENVOLVIMENTO** — Esta ferramenta está em desenvolvimento ativo e pode gerar overlays incompletos ou imprecisos. Use por sua conta e risco.

---

# OBS Scene to Overlay Exporter

App desktop que converte suas cenas do OBS Studio em overlays HTML auto-contidos — prontos para hospedar em qualquer servidor estático e usar como browser source em serviços de OBS na nuvem (ex: Antiscuff).

## O que faz

1. Lê o arquivo `.json` da sua coleção de cenas do OBS
2. Permite escolher qual cena exportar
3. Copia todos os arquivos locais (vídeos, imagens, GIFs) para uma pasta `assets/`
4. Baixa equivalentes do Google Fonts para fontes do sistema Windows (WOFF2 auto-hospedado)
5. Gera um `index.html` que replica o layout da sua cena OBS com precisão de pixels

A pasta gerada pode ser enviada para **Netlify, Vercel, GitHub Pages** ou qualquer host estático — e então usada como URL de browser source em serviços de OBS na nuvem.

## Tipos de source suportados

| Source OBS | Output HTML |
|---|---|
| Imagem | `<img>` |
| Vídeo (mp4/webm) | `<canvas>` com chroma key em JS |
| GIF | `<img>` |
| Texto (GDI+) | `<div>` com fonte, cor, contorno |
| Browser Source | `<iframe>` |
| Color Source | `<div>` com cor de fundo |
| Grupo | `<div>` recursivo |
| Cena aninhada | `<div>` recursivo |
| Áudio | Ignorado (sem elemento visual) |

## Como usar

1. No OBS Studio: **Coleção de Cenas → Exportar**
2. Abra o `obs-overlay-exporter.exe`
3. Selecione ou arraste o arquivo `.json` exportado
4. Escolha a cena na lista
5. Clique em **Exportar Overlay**
6. Envie a pasta gerada `cenas/{nome-da-cena}/` para seu host estático
7. Use a URL hospedada como Browser Source no seu OBS na nuvem

## Compilar do código fonte

```bash
cd src
cargo build --release
# output: src/target/release/obs-overlay-exporter.exe
```

**Requisito:** Rust toolchain (https://rustup.rs)

## Limitações conhecidas

- O chroma key em vídeos funciona via canvas JS — requer servir o HTML via HTTP (não `file://`) para fontes de vídeo cross-origin
- Fontes do sistema Windows (ex: OCR A Extended) são substituídas por equivalentes do Google Fonts quando não disponíveis no servidor
- Alguns efeitos complexos de filtros do OBS ainda não são suportados
- Grupos são um quirk conhecido no formato JSON do OBS e podem ter comportamento inesperado em casos extremos

## Licença

MIT
