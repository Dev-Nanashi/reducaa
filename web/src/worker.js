/**
 * Reducaa Web Worker
 *
 * Runs the WASM compression engine off the main thread so the UI stays
 * smooth. Communicates with main.js via postMessage.
 */

import init, { initReducaa, compress, detectFormat, inspectImage } from './wasm/reducaa_wasm.js';

let wasmReady = false;

async function initWasm() {
  try {
    await init();
    initReducaa();
    wasmReady = true;
    self.postMessage({ type: 'ready' });
  } catch (err) {
    self.postMessage({ type: 'error', error: `WASM init failed: ${err.message}` });
  }
}

self.onmessage = async (event) => {
  const { id, type, data, options } = event.data;

  if (type === 'inspect') {
    if (!wasmReady) {
      self.postMessage({ id, type: 'error', error: 'WASM not ready' });
      return;
    }
    try {
      const bytes = new Uint8Array(data);
      const info = inspectImage(bytes);
      self.postMessage({ id, type: 'inspect_result', info });
    } catch (err) {
      self.postMessage({ id, type: 'error', error: err.message || String(err) });
    }
    return;
  }

  if (type === 'compress') {
    if (!wasmReady) {
      self.postMessage({ id, type: 'error', error: 'WASM engine not ready yet' });
      return;
    }

    try {
      const bytes = new Uint8Array(data);
      const detectedFormat = detectFormat(bytes);

      // Build compression options
      const wasmOptions = {
        quality: options.quality,
        format: options.format && options.format !== 'auto' ? options.format : null,
        maxWidth: options.maxWidth || null,
        maxHeight: options.maxHeight || null,
        keepAspectRatio: options.keepAspectRatio !== false,
        preserveMetadata: options.preserveMetadata === true,
        lossless: options.lossless === true,
      };

      // Run compression in Rust
      const result = compress(bytes, wasmOptions);

      // Ensure Uint8Array
      let resultData = result.data instanceof Uint8Array ? result.data : new Uint8Array(result.data);

      self.postMessage({
        id,
        type: 'result',
        result: {
          data: resultData.buffer,
          format: result.format,
          detectedInputFormat: detectedFormat,
          originalSize: result.originalSize,
          compressedSize: result.compressedSize,
          width: result.width,
          height: result.height,
          reductionPercent: result.reductionPercent,
          preserveMetadata: wasmOptions.preserveMetadata,
        },
      }, [resultData.buffer]);
    } catch (err) {
      self.postMessage({
        id,
        type: 'error',
        error: err.message || String(err),
      });
    }
  }
};

// Boot up
initWasm();
