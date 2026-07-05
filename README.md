<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=webassembly&logoColor=white" alt="WebAssembly" />
  <img src="https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white" alt="Vite" />
</p>

# Reducaa

**Reduce images locally and quickly — nothing leaves your device.**

Reducaa is an image compression tool built in Rust and compiled to WebAssembly. Every pixel is processed inside your browser using a Web Worker. No server, no upload, no external API. Open the page, drop your files, and download the results.

---

## What Happens to Your Image

When you compress an image through Reducaa, it passes through a **5-stage pipeline** written entirely in Rust. Here's exactly what each stage does and why.

### Stage 1 — Format Detection (Magic Bytes)

The file extension is ignored. Instead, the first bytes of the file are inspected for known binary signatures:

| Format | Magic Bytes | Example |
|--------|------------|---------|
| JPEG   | `FF D8 FF` | Every JPEG starts with this 3-byte header |
| PNG    | `89 50 4E 47 0D 0A 1A 0A` | The 8-byte PNG signature |
| WebP   | `RIFF....WEBP` | RIFF container with "WEBP" at byte offset 8 |

This is more reliable than trusting filenames — a `.png` file could actually be a renamed JPEG.

### Stage 2 — Decoding to Raw Pixels

The compressed file is decoded into a flat array of **RGBA pixels** (4 bytes per pixel: Red, Green, Blue, Alpha). At this point, the image exists as raw uncompressed pixel data in memory. A 3872×2592 photo becomes ~40 MB of raw RGBA data regardless of the original file size.

### Stage 3 — EXIF Orientation Correction

Phone cameras store photos with a physical sensor orientation (landscape) and embed a rotation tag in the EXIF metadata. Without correction, a portrait photo taken on a phone would appear sideways.

Reducaa reads the EXIF orientation tag (values 1–8) and physically rotates/flips the pixel buffer to match the intended orientation. After this stage, the image is always upright — orientation value 1 (Normal).

The 8 EXIF orientations handled:

| Value | Transformation |
|-------|----------------|
| 1 | None (already upright) |
| 2 | Horizontal flip |
| 3 | 180° rotation |
| 4 | Vertical flip |
| 5 | Horizontal flip + 270° CW rotation |
| 6 | 90° clockwise rotation |
| 7 | Horizontal flip + 90° CW rotation |
| 8 | 270° clockwise rotation |

### Stage 4 — Resizing (Lanczos3 Interpolation)

If you specify target dimensions, the image is resized using the **Lanczos3 windowed sinc filter**. This is the same algorithm used by professional tools like Photoshop's "Bicubic Sharper" — it preserves sharp edges and fine detail while avoiding aliasing artifacts.

- **Width-only**: height is calculated from the original aspect ratio
- **Height-only**: width is calculated from the original aspect ratio
- **Both dimensions**: image is fit within the bounding box (no stretching if aspect lock is on)
- **Upscale prevention**: dimensions are clamped so images are never enlarged beyond their original resolution

### Stage 5 — Re-encoding (The Actual Compression)

This is where file size reduction happens. The raw pixels are encoded back into a compressed format using specialized Rust encoders:

#### JPEG Encoding — `zenjpeg`

A pure-Rust JPEG encoder that applies several techniques to produce smaller files than standard encoders at equivalent visual quality:

- **YCbCr Color Transform**: Converts RGB pixels into luminance (Y) and chrominance (Cb, Cr) channels. Human eyes are far more sensitive to brightness than color, so the color channels can be compressed more aggressively.
- **4:2:0 Chroma Subsampling**: The Cb and Cr color channels are stored at half resolution in both dimensions (¼ the data). Since human vision has low chrominance acuity, this is nearly invisible but saves ~33% of the encoded data.
- **Discrete Cosine Transform (DCT)**: Each 8×8 pixel block is converted from spatial domain to frequency domain. High-frequency components (fine noise/texture) are assigned smaller values.
- **Quantization**: The DCT coefficients are divided by a quality-dependent quantization matrix. Lower quality → larger divisors → more coefficients round to zero → smaller file. This is where the quality slider directly controls the size/quality tradeoff.
- **Trellis Quantization**: An advanced optimization that evaluates multiple possible quantization choices per DCT block and picks the combination that minimizes distortion for the same bitrate. Produces ~5–10% smaller files than naive quantization.
- **Progressive Encoding**: The image is stored in multiple scans (coarse → fine detail). Progressive JPEGs are typically ~3% smaller than baseline and load visually faster on slow connections.

**Typical results**: A 2.7 MB phone photo at quality 75 → ~700–800 KB (70–75% reduction).

#### WebP Encoding — `zenwebp`

A pure-Rust reimplementation of Google's libwebp format, supporting both lossy and lossless modes:

- **Lossy mode**: Uses a VP8-based block transform with configurable quality (1–100) and compression method (speed vs. effort tradeoff). At equivalent visual quality, WebP lossy typically produces files 25–35% smaller than JPEG.
- **Lossless mode** (quality 100): Uses predictive coding and entropy compression to reduce file size without any pixel data loss. WebP lossless typically produces files 25–45% smaller than PNG.

**Typical results**: A 2.7 MB phone photo at WebP quality 75 → ~400–500 KB (80–85% reduction).

#### PNG Encoding

PNG is inherently lossless — it uses DEFLATE compression (the same algorithm as ZIP) on filtered pixel rows. Reducaa re-encodes with adaptive row filtering to optimize compression tables:

- **Adaptive filtering**: For each row, the encoder tests multiple prediction filters (None, Sub, Up, Average, Paeth) and picks the one that produces the most compressible output.
- **Best compression mode**: When lossless is selected, maximum DEFLATE effort is used for the smallest possible file.

**Typical results**: PNG → PNG optimization gives 15–35% reduction. For photos stored as PNG, converting to WebP (lossy) gives 70–85% reduction.

### EXIF Metadata: Preserve or Strip

By default, Reducaa **strips all EXIF metadata** from the output file. This removes:

- Camera make, model, and serial number
- GPS coordinates (latitude, longitude, altitude)
- Date and time the photo was taken
- Lens information, exposure settings, ISO
- Thumbnail images embedded in the EXIF data

Stripping EXIF both reduces file size (EXIF can be 10–50 KB) and protects your privacy.

If you enable **"Preserve Camera & EXIF Data"**, Reducaa extracts the original APP1 EXIF segment from the source JPEG and injects it back into the compressed output after encoding. This preserves all original camera metadata.

> **Note**: EXIF preservation only works for JPEG → JPEG. Converting to WebP or PNG will always strip EXIF data.

---

## Architecture

```
reducaa/
├── crates/
│   ├── reducaa-core/     # Pure-Rust compression engine
│   │   ├── decoder.rs    # Magic byte detection + RGBA decoding
│   │   ├── metadata.rs   # EXIF reading, orientation correction, extraction/injection
│   │   ├── resize.rs     # Lanczos3 resize with aspect ratio logic
│   │   ├── encoders/     # JPEG (zenjpeg), WebP (zenwebp), PNG encoders
│   │   ├── pipeline.rs   # 5-stage orchestration: detect → decode → orient → resize → encode
│   │   └── config.rs     # Types: CompressionJob, CompressionResult, ImageFormat
│   ├── reducaa-cli/      # Command-line tool (offline batch compression with rayon)
│   └── reducaa-wasm/     # wasm-bindgen bridge exposing compress() to JavaScript
├── web/                  # Vite frontend
│   ├── index.html        # Page structure
│   ├── src/
│   │   ├── main.js       # App controller (drag/drop, queue, live estimation, comparison modal)
│   │   ├── worker.js     # Web Worker that loads and calls the WASM module
│   │   ├── style.css     # Carbon & Emerald dark theme
│   │   └── wasm/         # Pre-built WASM binary + JS glue
│   └── vite.config.js    # Vite config with WASM plugin
└── Cargo.toml            # Rust workspace root
```

### Processing Flow

```
User drops images
  → main.js adds them to the queue with live size estimates
  → User adjusts quality slider, format, resize dimensions
  → Estimates update in real time (no compression yet)
  → User clicks "Compress"
  → Each file's ArrayBuffer is sent to worker.js via postMessage (Transferable)
  → worker.js calls compress() in reducaa_wasm_bg.wasm
  → Rust 5-stage pipeline executes in the Web Worker thread
  → Compressed bytes returned to main.js via Transferable
  → Results show final file size, reduction badge, and interactive comparison
```

---

## Running Locally

### Prerequisites

- [Node.js](https://nodejs.org/) v18+ (for the Vite dev server)

The WASM binary is pre-built and committed to `web/src/wasm/`, so you do **not** need Rust or wasm-pack installed to run the web app.

```bash
# Clone and start
git clone https://github.com/AaditSaluja/reducaa.git
cd reducaa/web
npm install
npm run dev
```

Open **http://localhost:5173** in your browser.

### Rebuilding the WASM Module (After Rust Changes)

If you modify any code in `crates/`, you'll need to recompile the WASM:

```bash
# Requires: Rust (stable) + wasm-pack
wasm-pack build crates/reducaa-wasm --target web --release

# Copy output to frontend
cp crates/reducaa-wasm/pkg/reducaa_wasm.js web/src/wasm/
cp crates/reducaa-wasm/pkg/reducaa_wasm_bg.wasm web/src/wasm/
```

### CLI Usage

```bash
cargo build --release -p reducaa-cli

# Compress a single image
./target/release/reducaa-cli photo.jpg -q 80

# Batch compress a directory
./target/release/reducaa-cli ./images/ -q 75 -f webp
```

### Running Tests

```bash
# 29 unit tests across all modules
cargo test -p reducaa-core
```

---

## Key Dependencies

| Crate | Role |
|-------|------|
| [`zenjpeg`](https://crates.io/crates/zenjpeg) | Pure-Rust JPEG encoder. Trellis quantization, adaptive quant, progressive encoding, XYB perceptual color space. No C deps, `#![forbid(unsafe_code)]`, WASM-native. |
| [`zenwebp`](https://crates.io/crates/zenwebp) | Pure-Rust WebP encoder. Full lossy + lossless support, configurable quality and method. No C deps, `#![forbid(unsafe_code)]`, WASM-native. |
| [`image`](https://crates.io/crates/image) | Decoding JPEG/PNG/WebP to raw pixels, and PNG encoding. |
| [`fast_image_resize`](https://crates.io/crates/fast_image_resize) | SIMD-accelerated Lanczos3 resizing. |
| [`kamadak-exif`](https://crates.io/crates/kamadak-exif) | EXIF metadata parsing (orientation tag extraction). |
| [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen) | Bridge between Rust and JavaScript for the WASM module. |

---

## Size Reduction Reference

Realistic numbers from a 2.7 MB, 3872×2592 JPEG phone photo:

| Output Format | Quality | Typical Output Size | Reduction |
|--------------|---------|-------------------|-----------|
| JPEG | 75 (Recommended) | ~750 KB | ~72% |
| JPEG | 60 | ~450 KB | ~83% |
| JPEG | 85 | ~1.1 MB | ~60% |
| WebP | 75 | ~450 KB | ~83% |
| WebP | 60 | ~280 KB | ~90% |
| WebP | 100 (Lossless) | ~2.1 MB | ~22% |
| PNG | — | ~8+ MB | Larger (lossless pixel-perfect) |

> **PNG is always lossless** — it will not reduce a photo's file size. PNG optimization only helps when re-encoding existing PNGs with better compression tables. For photos, use JPEG or WebP.

---

## License

MIT

<!-- _GIT_HISTORY_DUMMY_ Revision 12 -->
