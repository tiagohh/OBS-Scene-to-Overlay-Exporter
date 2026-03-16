> ⚠️ **EM DESENVOLVIMENTO** — Esta ferramenta está em desenvolvimento ativo e pode gerar overlays incompletos ou imprecisos. Use por sua conta e risco.

---

# OBS Scene to Overlay Exporter

App desktop que converte suas cenas do OBS Studio em overlays HTML auto-contidos — prontos para hospedar em qualquer servidor estático e usar como browser source em serviços de OBS na nuvem (ex: Antiscuff).

## O que faz

1. Lê o arquivo `.json` da sua coleção de cenas do OBS
2. Permite escolher qual cena exportar
3. Copia todos os arquivos locais (vídeos, imagens, GIFs) para uma pasta `assets/`
4. Converte vídeos com chroma key para WebM VP9 com canal alpha (requer ffmpeg)
5. Baixa equivalentes do Google Fonts para fontes do sistema Windows (WOFF2 auto-hospedado)
6. Gera um `index.html` que replica o layout da sua cena OBS com precisão de pixels

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

## ffmpeg (opcional, recomendado)

O ffmpeg permite a conversão automática de vídeos com chroma key para WebM VP9 com canal alpha, gerando um resultado mais limpo.

**Instalação (Windows):**
1. Baixe o **ffmpeg essentials build** em [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) → `ffmpeg-release-essentials.zip`
2. Extraia em `C:\ffmpeg\`
3. Adicione ao PATH: abra **Propriedades do Sistema → Avançado → Variáveis de Ambiente**, edite **Path** em Variáveis do Sistema, adicione `C:\ffmpeg\<nome-da-pasta>\bin`
4. Verifique: abra um novo terminal e execute `ffmpeg -version`

Sem o ffmpeg, vídeos com chroma key são copiados normalmente e o canvas JS ainda remove o fundo verde via HTTP.

## Compilar do código fonte

```bash
cargo build --release
# output: target/release/obs-overlay-exporter.exe
```

**Requisito:** Rust toolchain (https://rustup.rs)

## Limitações conhecidas

- O chroma key em vídeos requer servir via HTTP (não `file://`) — um fundo esverdeado é esperado ao abrir o HTML diretamente do disco
- Fontes do sistema Windows (ex: OCR A Extended) são usadas diretamente se disponíveis na máquina que abrir o HTML; equivalentes do Google Fonts são baixados como fallback para servidores remotos
- Browser sources (Chat Box do Streamlabs, alertas do StreamElements, etc.) são incorporados como `<iframe>` e dependem totalmente desses serviços externos estarem online e permitindo incorporação. Erros de carregamento de fontes ou falhas visuais nesses widgets são causados pelo serviço externo, não por esta ferramenta
- Alguns efeitos complexos de filtros do OBS ainda não são suportados

## Licença

MIT
