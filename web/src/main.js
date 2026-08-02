/**
 * Reducaa — Main Application Controller
 *
 * Implements:
 * - Live Pre-Compression Size & Savings Estimation
 * - Direct Quality Slider with labeled stops
 * - EXIF preservation toggle with clear visual status
 * - Interactive Split-View Before/After Comparison Modal with clip-path
 * - Accurate reduction / expansion analytics
 */

// ── State ─────────────────────────────────────────────────────────────

let worker = null;
let workerReady = false;
let requestId = 0;
const pendingRequests = new Map();

/** Queued files waiting to be processed */
let queuedFiles = [];
/** Processed compression results */
const compressedResults = [];

// User configurable settings
const settings = {
  quality: 75,
  format: 'auto',
  maxWidth: null,
  maxHeight: null,
  keepAspectRatio: true,
  preserveMetadata: false, // Default: false (privacy mode — EXIF stripped)
};

// ── DOM References ────────────────────────────────────────────────────

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const dropZone = $('#drop-zone');
const fileInput = $('#file-input');
const queueSection = $('#queue-section');
const queueList = $('#queue-list');
const queueCount = $('#queue-count');
const btnAddMore = $('#btn-add-more');
const controlsSection = $('#controls-section');
const btnCompress = $('#btn-compress');
const btnCompressText = $('#btn-compress-text');
const resultsSection = $('#results-section');
const fileList = $('#file-list');
const qualitySlider = $('#quality-slider');
const qualityDisplay = $('#quality-display');
const formatPicker = $('#format-picker');
const maxWidthInput = $('#max-width');
const maxHeightInput = $('#max-height');
const btnLockAspect = $('#btn-lock-aspect');
const preserveMetadataCheckbox = $('#preserve-metadata-checkbox');
const metaStatusPill = $('#meta-status-pill');
const btnDownloadAll = $('#btn-download-all');
const btnClear = $('#btn-clear');
const loadingOverlay = $('#loading-overlay');

const statTotalFiles = $('#stat-total-files');
const statTotalSaved = $('#stat-total-saved');
const statTotalSavedPct = $('#stat-total-saved-pct');
const statTotalOutput = $('#stat-total-output');
const statTotalOutputOrig = $('#stat-total-output-orig');

// Modal Elements
const compareModal = $('#compare-modal');
const compareBackdrop = $('#compare-backdrop');
const compareClose = $('#compare-close');
const compareFilename = $('#compare-filename');
const compareBeforeImg = $('#compare-before-img');
const compareAfterImg = $('#compare-after-img');
const splitContainer = $('#split-container');
const compareDownloadBtn = $('#compare-download-btn');
const compOrigSize = $('#comp-orig-size');
const compOrigMeta = $('#comp-orig-meta');
const compNewSize = $('#comp-new-size');
const compNewMeta = $('#comp-new-meta');
const compDiffVal = $('#comp-diff-val');
const compDiffSub = $('#comp-diff-sub');
const compExifStatus = $('#comp-exif-status');
const compExifDetail = $('#comp-exif-detail');

// ── Web Worker Initialization ─────────────────────────────────────────

function initWorker() {
  worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

  worker.onmessage = (event) => {
    const msg = event.data;

    if (msg.type === 'ready') {
      workerReady = true;
      loadingOverlay.classList.add('hidden');
      return;
    }

    if (msg.type === 'result' || msg.type === 'error') {
      const pending = pendingRequests.get(msg.id);
      if (pending) {
        pendingRequests.delete(msg.id);
        if (msg.type === 'result') {
          pending.resolve(msg.result);
        } else {
          pending.reject(new Error(msg.error));
        }
      }
    }
  };

  worker.onerror = (err) => {
    console.error('Worker error:', err);
    loadingOverlay.classList.add('hidden');
  };
}

function runWasmCompress(arrayBuffer, options) {
  return new Promise((resolve, reject) => {
    const id = ++requestId;
    pendingRequests.set(id, { resolve, reject });
    worker.postMessage(
      { id, type: 'compress', data: arrayBuffer, options },
      [arrayBuffer]
    );
  });
}

// ── Size Estimation Engine ────────────────────────────────────────────

function estimateCompressedSize(fileSize, width, height, format, quality, targetW, targetH) {
  let w = targetW || width || 1920;
  let h = targetH || height || 1080;

  if (targetW && !targetH && width && height) {
    h = Math.round(targetW * (height / width));
  } else if (targetH && !targetW && width && height) {
    w = Math.round(targetH * (width / height));
  }

  const pixels = w * h;
  const targetFormat = format === 'auto' ? 'jpeg' : format;

  let estBytes = 0;
  if (targetFormat === 'webp') {
    // WebP: ~0.046 bytes/pixel at Q75
    const bpp = 0.012 + Math.pow(quality / 100, 2.5) * 0.07;
    estBytes = pixels * bpp;
  } else if (targetFormat === 'jpeg' || targetFormat === 'jpg') {
    // JPEG: ~0.073 bytes/pixel at Q75
    const bpp = 0.018 + Math.pow(quality / 100, 2.8) * 0.125;
    estBytes = pixels * bpp;
  } else {
    // PNG
    estBytes = Math.min(fileSize, pixels * 0.35);
  }

  estBytes = Math.max(1024, Math.min(estBytes, fileSize * 1.2));
  const delta = fileSize - estBytes;
  const pct = (delta / fileSize) * 100;

  return {
    estSize: Math.round(estBytes),
    delta: Math.round(delta),
    reductionPct: pct,
  };
}

// ── Quality Slider & Ticks Logic ──────────────────────────────────────

function updateQuality(val) {
  settings.quality = parseInt(val, 10);
  qualitySlider.value = settings.quality;

  let label = `${settings.quality}%`;
  if (settings.quality === 75) label = '75% (Recommended)';
  else if (settings.quality >= 100) label = '100% (Lossless)';
  else if (settings.quality <= 25) label = `${settings.quality}% (Max Compression)`;

  qualityDisplay.textContent = label;

  // Highlight the closest tick label
  let closestTick = null;
  let minDiff = Infinity;
  $$('.tick-label').forEach((tick) => {
    const tickVal = parseInt(tick.dataset.val, 10);
    const diff = Math.abs(settings.quality - tickVal);
    if (diff < minDiff) {
      minDiff = diff;
      closestTick = tick;
    }
  });

  $$('.tick-label').forEach((tick) => {
    tick.classList.toggle('active', tick === closestTick);
  });

  updateLiveEstimates();
}

qualitySlider.addEventListener('input', (e) => {
  updateQuality(e.target.value);
});

$$('.tick-label').forEach((tick) => {
  tick.addEventListener('click', () => {
    updateQuality(tick.dataset.val);
  });
});

// ── Metadata Toggle Logic ─────────────────────────────────────────────

preserveMetadataCheckbox.addEventListener('change', () => {
  settings.preserveMetadata = preserveMetadataCheckbox.checked;
  if (settings.preserveMetadata) {
    metaStatusPill.className = 'meta-status-pill preserved';
    metaStatusPill.textContent = 'EXIF Preserved (Camera & GPS Kept)';
  } else {
    metaStatusPill.className = 'meta-status-pill stripped';
    metaStatusPill.textContent = 'EXIF Removed (Privacy Mode)';
  }
  updateLiveEstimates();
});

// ── Drag & Drop & File Picker ─────────────────────────────────────────

let dragCounter = 0;

dropZone.addEventListener('dragenter', (e) => {
  e.preventDefault();
  dragCounter++;
  dropZone.classList.add('drag-over');
});

dropZone.addEventListener('dragleave', (e) => {
  e.preventDefault();
  dragCounter--;
  if (dragCounter <= 0) {
    dragCounter = 0;
    dropZone.classList.remove('drag-over');
  }
});

dropZone.addEventListener('dragover', (e) => {
  e.preventDefault();
});

dropZone.addEventListener('drop', (e) => {
  e.preventDefault();
  dragCounter = 0;
  dropZone.classList.remove('drag-over');

  const files = Array.from(e.dataTransfer.files).filter(isImageFile);
  if (files.length > 0) addFilesToQueue(files);
});

dropZone.addEventListener('click', () => fileInput.click());
dropZone.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    fileInput.click();
  }
});

fileInput.addEventListener('change', () => {
  const files = Array.from(fileInput.files).filter(isImageFile);
  if (files.length > 0) addFilesToQueue(files);
  fileInput.value = '';
});

btnAddMore.addEventListener('click', () => fileInput.click());

function isImageFile(file) {
  return ['image/jpeg', 'image/png', 'image/webp'].includes(file.type);
}

// ── Queue Management ──────────────────────────────────────────────────

async function addFilesToQueue(files) {
  for (const file of files) {
    const exists = queuedFiles.some((f) => f.file.name === file.name && f.file.size === file.size);
    if (!exists) {
      const origDims = await getImageDimensions(file);
      queuedFiles.push({
        file,
        dims: origDims,
        origUrl: URL.createObjectURL(file),
      });
    }
  }

  renderQueue();
}

function removeFileFromQueue(index) {
  if (queuedFiles[index]?.origUrl) {
    URL.revokeObjectURL(queuedFiles[index].origUrl);
  }
  queuedFiles.splice(index, 1);
  renderQueue();
}

function renderQueue() {
  if (queuedFiles.length === 0) {
    queueSection.classList.add('hidden');
    controlsSection.classList.add('hidden');
    return;
  }

  queueSection.classList.remove('hidden');
  controlsSection.classList.remove('hidden');
  queueCount.textContent = `${queuedFiles.length}`;

  queueList.innerHTML = '';
  queuedFiles.forEach((item, idx) => {
    const el = document.createElement('div');
    el.className = 'queue-item';
    el.id = `q-item-${idx}`;

    const dimsText = item.dims ? `${item.dims.width}×${item.dims.height}` : '';
    const est = estimateCompressedSize(
      item.file.size,
      item.dims?.width,
      item.dims?.height,
      settings.format,
      settings.quality,
      settings.maxWidth,
      settings.maxHeight
    );

    let estPillHtml = '';
    if (est.reductionPct > 0) {
      estPillHtml = `<span class="queue-est-pill">-${est.reductionPct.toFixed(0)}%</span>`;
    }

    el.innerHTML = `
      <img src="${item.origUrl}" class="queue-thumb" alt="" />
      <div class="queue-info">
        <div class="queue-name" title="${escapeHtml(item.file.name)}">${escapeHtml(item.file.name)}</div>
        <div class="queue-meta">
          <span>${formatBytes(item.file.size)}</span>
          ${dimsText ? `<span>•</span><span>${dimsText}</span>` : ''}
        </div>
        <div class="queue-est-row">
          <span class="queue-est-label">Est. Final:</span>
          <span class="queue-est-val">~${formatBytes(est.estSize)}</span>
          ${estPillHtml}
        </div>
      </div>
      <button class="queue-remove" data-idx="${idx}" title="Remove file">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    `;

    queueList.appendChild(el);
  });

  queueList.querySelectorAll('.queue-remove').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      removeFileFromQueue(parseInt(btn.dataset.idx, 10));
    });
  });

  updateLiveEstimates();
}

function updateLiveEstimates() {
  if (queuedFiles.length === 0) return;

  let totalEstBytes = 0;
  let totalOrigBytes = 0;

  queuedFiles.forEach((item, idx) => {
    const est = estimateCompressedSize(
      item.file.size,
      item.dims?.width,
      item.dims?.height,
      settings.format,
      settings.quality,
      settings.maxWidth,
      settings.maxHeight
    );

    totalOrigBytes += item.file.size;
    totalEstBytes += est.estSize;

    const el = document.getElementById(`q-item-${idx}`);
    if (el) {
      const valEl = el.querySelector('.queue-est-val');
      const pillEl = el.querySelector('.queue-est-pill');
      if (valEl) valEl.textContent = `~${formatBytes(est.estSize)}`;
      if (pillEl) {
        if (est.reductionPct > 0) {
          pillEl.textContent = `-${est.reductionPct.toFixed(0)}%`;
          pillEl.style.display = 'inline-block';
        } else {
          pillEl.style.display = 'none';
        }
      }
    }
  });

  const totalSavedEst = Math.max(0, totalOrigBytes - totalEstBytes);
  const totalSavedPct = totalOrigBytes > 0 ? (totalSavedEst / totalOrigBytes) * 100 : 0;

  btnCompressText.textContent = `Compress ${queuedFiles.length} Image${queuedFiles.length > 1 ? 's' : ''} (Est. ~${formatBytes(totalEstBytes)}${totalSavedPct > 5 ? `, -${totalSavedPct.toFixed(0)}%` : ''})`;
}

// ── Compression Settings Controls ─────────────────────────────────────

// Format picker
formatPicker.addEventListener('click', (e) => {
  const btn = e.target.closest('.format-btn');
  if (!btn) return;

  $$('.format-btn').forEach((b) => b.classList.remove('active'));
  btn.classList.add('active');
  settings.format = btn.dataset.format;
  updateLiveEstimates();
});

// Aspect Ratio Toggle
btnLockAspect.addEventListener('click', () => {
  settings.keepAspectRatio = !settings.keepAspectRatio;
  btnLockAspect.classList.toggle('active', settings.keepAspectRatio);
  btnLockAspect.querySelector('span').textContent = settings.keepAspectRatio ? 'Aspect Locked' : 'Aspect Free';
});

// Resize inputs with proportional auto-calculation if aspect locked
maxWidthInput.addEventListener('input', () => {
  const val = parseInt(maxWidthInput.value, 10);
  settings.maxWidth = val > 0 ? val : null;

  if (settings.keepAspectRatio && val > 0 && queuedFiles[0]?.dims) {
    const ratio = queuedFiles[0].dims.height / queuedFiles[0].dims.width;
    maxHeightInput.value = Math.round(val * ratio);
    settings.maxHeight = parseInt(maxHeightInput.value, 10);
  }
  updateLiveEstimates();
});

maxHeightInput.addEventListener('input', () => {
  const val = parseInt(maxHeightInput.value, 10);
  settings.maxHeight = val > 0 ? val : null;

  if (settings.keepAspectRatio && val > 0 && queuedFiles[0]?.dims) {
    const ratio = queuedFiles[0].dims.width / queuedFiles[0].dims.height;
    maxWidthInput.value = Math.round(val * ratio);
    settings.maxWidth = parseInt(maxWidthInput.value, 10);
  }
  updateLiveEstimates();
});

// ── Compression Execution ─────────────────────────────────────────────

btnCompress.addEventListener('click', async () => {
  if (queuedFiles.length === 0 || !workerReady) return;

  btnCompress.disabled = true;
  btnCompress.innerHTML = `
    <div class="loading-spinner" style="width:16px;height:16px;border-width:2px;"></div>
    <span>Compressing…</span>
  `;

  resultsSection.classList.remove('hidden');

  const itemsToProcess = [...queuedFiles];
  queuedFiles = [];
  renderQueue();

  for (const item of itemsToProcess) {
    const cardId = `res-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    renderProcessingCard(cardId, item.file);

    try {
      const arrayBuffer = await item.file.arrayBuffer();
      const options = {
        quality: settings.quality,
        format: settings.format === 'auto' ? null : settings.format,
        maxWidth: settings.maxWidth,
        maxHeight: settings.maxHeight,
        keepAspectRatio: settings.keepAspectRatio,
        preserveMetadata: settings.preserveMetadata,
        lossless: settings.quality >= 100,
      };

      const result = await runWasmCompress(arrayBuffer, options);

      // Create Blob for download & display
      const mimeType = getMimeType(result.format);
      const blob = new Blob([new Uint8Array(result.data)], { type: mimeType });
      const ext = getExtension(result.format);
      const outputName = item.file.name.replace(/\.[^.]+$/, '') + '_min.' + ext;
      const compUrl = URL.createObjectURL(blob);

      const record = {
        id: cardId,
        name: outputName,
        originalName: item.file.name,
        origUrl: item.origUrl,
        compUrl: compUrl,
        blob,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        reductionPercent: result.reductionPercent,
        origWidth: item.dims?.width || result.width,
        origHeight: item.dims?.height || result.height,
        width: result.width,
        height: result.height,
        format: result.format,
        preservedMetadata: settings.preserveMetadata,
      };

      compressedResults.push(record);
      updateSuccessCard(cardId, record);
    } catch (err) {
      console.error('Compression error for', item.file.name, err);
      updateErrorCard(cardId, err.message || 'Compression failed');
    }
  }

  updateSummaryStats();

  btnCompress.disabled = false;
  btnCompress.innerHTML = `
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
    <span id="btn-compress-text">Compress Images</span>
  `;
});

// Helper: reads original image dimensions via Image()
function getImageDimensions(file) {
  return new Promise((resolve) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      resolve({ width: img.naturalWidth, height: img.naturalHeight });
    };
    img.onerror = () => {
      resolve(null);
    };
    img.src = url;
  });
}

// ── Results UI Rendering ──────────────────────────────────────────────

function renderProcessingCard(id, file) {
  const thumbUrl = URL.createObjectURL(file);
  const card = document.createElement('div');
  card.className = 'file-item';
  card.id = id;

  card.innerHTML = `
    <div class="file-thumb-wrap"><img src="${thumbUrl}" alt="" /></div>
    <div class="file-info">
      <div class="file-name" title="${escapeHtml(file.name)}">${escapeHtml(file.name)}</div>
      <div class="file-meta">
        <span>${formatBytes(file.size)}</span>
      </div>
    </div>
    <div class="file-badge neutral">Compressing…</div>
    <div class="file-actions"></div>
  `;

  fileList.prepend(card);
}

function updateSuccessCard(id, data) {
  const card = document.getElementById(id);
  if (!card) return;

  const originalSize = data.originalSize;
  const compressedSize = data.compressedSize;
  const delta = originalSize - compressedSize;

  let badgeHtml = '';
  let hintHtml = '';

  if (delta > 0) {
    const percent = (delta / originalSize) * 100;
    badgeHtml = `
      <div class="file-badge-wrap">
        <span class="file-final-size">${formatBytes(compressedSize)}</span>
        <span class="file-badge reduction">-${percent.toFixed(0)}%</span>
      </div>
    `;
  } else if (delta < 0) {
    const percent = (Math.abs(delta) / originalSize) * 100;
    badgeHtml = `
      <div class="file-badge-wrap">
        <span class="file-final-size">${formatBytes(compressedSize)}</span>
        <span class="file-badge increase">+${percent.toFixed(0)}%</span>
      </div>
    `;
    hintHtml = `<div class="file-hint">Original was heavily compressed. Lower quality (e.g. 50–65) or select WebP to reduce size.</div>`;
  } else {
    badgeHtml = `
      <div class="file-badge-wrap">
        <span class="file-final-size">${formatBytes(compressedSize)}</span>
        <span class="file-badge neutral">0%</span>
      </div>
    `;
  }

  card.innerHTML = `
    <div class="file-thumb-wrap" title="Click to compare Before & After">
      <img src="${data.compUrl}" alt="" />
      <div class="thumb-hover-overlay">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
      </div>
    </div>
    <div class="file-info">
      <div class="file-name" title="Click to view interactive comparison">${escapeHtml(data.name)}</div>
      <div class="file-meta">
        <span class="file-size-change">
          <span class="size-original">${formatBytes(originalSize)}</span>
          <span class="size-arrow">→</span>
          <span class="size-compressed">${formatBytes(compressedSize)}</span>
        </span>
        <span>•</span>
        <span>${data.width}×${data.height}</span>
        <span>•</span>
        <span>${data.format}</span>
      </div>
      ${hintHtml}
    </div>
    ${badgeHtml}
    <div class="file-actions">
      <button class="btn btn-secondary btn-sm btn-compare-item" title="Compare visual quality">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
        Compare
      </button>
      <a href="${data.compUrl}" download="${escapeHtml(data.name)}" class="btn btn-primary btn-sm" title="Download">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      </a>
    </div>
  `;

  const openCompare = () => openComparisonModal(data);
  card.querySelector('.file-thumb-wrap').addEventListener('click', openCompare);
  card.querySelector('.file-name').addEventListener('click', openCompare);
  card.querySelector('.btn-compare-item').addEventListener('click', openCompare);
}

function updateErrorCard(id, errorMsg) {
  const card = document.getElementById(id);
  if (!card) return;

  const badge = card.querySelector('.file-badge');
  if (badge) {
    badge.className = 'file-badge error';
    badge.textContent = 'Error';
  }

  const meta = card.querySelector('.file-meta');
  if (meta) {
    meta.innerHTML = `<span style="color:var(--red);">${escapeHtml(errorMsg)}</span>`;
  }
}

function updateSummaryStats() {
  const done = compressedResults;
  statTotalFiles.textContent = done.length;

  const totalOriginal = done.reduce((sum, f) => sum + f.originalSize, 0);
  const totalCompressed = done.reduce((sum, f) => sum + f.compressedSize, 0);
  const totalSaved = totalOriginal - totalCompressed;

  if (totalSaved >= 0) {
    statTotalSaved.className = 'stat-value highlight';
    statTotalSaved.textContent = formatBytes(totalSaved);
    const pct = totalOriginal > 0 ? (totalSaved / totalOriginal) * 100 : 0;
    statTotalSavedPct.textContent = `${pct.toFixed(1)}% size reduction`;
  } else {
    statTotalSaved.className = 'stat-value warning';
    statTotalSaved.textContent = `+${formatBytes(Math.abs(totalSaved))}`;
    statTotalSavedPct.textContent = 'File size increased';
  }

  statTotalOutput.className = 'stat-value highlight';
  statTotalOutput.textContent = formatBytes(totalCompressed);
  statTotalOutputOrig.textContent = `from ${formatBytes(totalOriginal)}`;
}

// ── Interactive Split Comparison Modal ────────────────────────────────

function openComparisonModal(data) {
  compareFilename.textContent = data.originalName;
  compareBeforeImg.src = data.origUrl;
  compareAfterImg.src = data.compUrl;

  compareDownloadBtn.href = data.compUrl;
  compareDownloadBtn.download = data.name;

  compOrigSize.textContent = formatBytes(data.originalSize);
  compOrigMeta.textContent = `${data.origWidth}×${data.origHeight} px`;

  compNewSize.textContent = formatBytes(data.compressedSize);
  compNewMeta.textContent = `${data.width}×${data.height} px • ${data.format}`;

  const delta = data.originalSize - data.compressedSize;
  if (delta > 0) {
    const percent = (delta / data.originalSize) * 100;
    compDiffVal.textContent = formatBytes(data.compressedSize);
    compDiffSub.textContent = `Reduced from ${formatBytes(data.originalSize)} (-${percent.toFixed(1)}%)`;
    compDiffVal.style.color = 'var(--emerald)';
  } else if (delta < 0) {
    const percent = (Math.abs(delta) / data.originalSize) * 100;
    compDiffVal.textContent = formatBytes(data.compressedSize);
    compDiffSub.textContent = `Expanded from ${formatBytes(data.originalSize)} (+${percent.toFixed(1)}%)`;
    compDiffVal.style.color = 'var(--amber)';
  } else {
    compDiffVal.textContent = formatBytes(data.compressedSize);
    compDiffSub.textContent = 'Exact same size as original';
    compDiffVal.style.color = 'var(--text-muted)';
  }

  if (data.preservedMetadata) {
    compExifStatus.textContent = 'Preserved';
    compExifStatus.style.color = 'var(--emerald)';
    compExifDetail.textContent = 'Camera & EXIF headers kept in output';
  } else {
    compExifStatus.textContent = 'Stripped';
    compExifStatus.style.color = 'var(--text-muted)';
    compExifDetail.textContent = 'EXIF tags removed for privacy';
  }

  setSplitPosition(50);

  compareModal.classList.remove('hidden');
  document.body.style.overflow = 'hidden';
}

function closeComparisonModal() {
  compareModal.classList.add('hidden');
  document.body.style.overflow = '';
}

compareClose.addEventListener('click', closeComparisonModal);
compareBackdrop.addEventListener('click', closeComparisonModal);
window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && !compareModal.classList.contains('hidden')) {
    closeComparisonModal();
  }
});

// Interactive Drag Slider for Split Comparison
let isDraggingSplit = false;

function setSplitPosition(pct) {
  const clampPct = Math.max(0, Math.min(100, pct));
  splitContainer.style.setProperty('--split-pos', `${clampPct}%`);
}

function handleSplitMove(clientX) {
  const rect = splitContainer.getBoundingClientRect();
  const x = clientX - rect.left;
  const pct = (x / rect.width) * 100;
  setSplitPosition(pct);
}

splitContainer.addEventListener('mousedown', (e) => {
  isDraggingSplit = true;
  handleSplitMove(e.clientX);
});

window.addEventListener('mousemove', (e) => {
  if (isDraggingSplit) {
    handleSplitMove(e.clientX);
  }
});

window.addEventListener('mouseup', () => {
  isDraggingSplit = false;
});

splitContainer.addEventListener('touchstart', (e) => {
  isDraggingSplit = true;
  if (e.touches[0]) handleSplitMove(e.touches[0].clientX);
});

window.addEventListener('touchmove', (e) => {
  if (isDraggingSplit && e.touches[0]) {
    handleSplitMove(e.touches[0].clientX);
  }
});

window.addEventListener('touchend', () => {
  isDraggingSplit = false;
});

// ── Batch ZIP Download ────────────────────────────────────────────────

btnDownloadAll.addEventListener('click', async () => {
  const files = compressedResults.filter((f) => f.blob);
  if (files.length === 0) return;

  if (files.length === 1) {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(files[0].blob);
    a.download = files[0].name;
    a.click();
    URL.revokeObjectURL(a.href);
    return;
  }

  btnDownloadAll.disabled = true;
  btnDownloadAll.innerHTML = `
    <div class="loading-spinner" style="width:14px;height:14px;border-width:2px;"></div>
    <span>Packaging ZIP…</span>
  `;

  try {
    const JSZip = (await import('jszip')).default;
    const zip = new JSZip();

    for (const file of files) {
      zip.file(file.name, file.blob);
    }

    const zipBlob = await zip.generateAsync({ type: 'blob' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(zipBlob);
    a.download = 'reducaa-compressed.zip';
    a.click();
    URL.revokeObjectURL(a.href);
  } catch (err) {
    console.error('ZIP generation error:', err);
    alert('Failed to package ZIP. Please download files individually.');
  } finally {
    btnDownloadAll.disabled = false;
    btnDownloadAll.innerHTML = `
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      <span>Download All (.ZIP)</span>
    `;
  }
});

// ── Clear Action ──────────────────────────────────────────────────────

btnClear.addEventListener('click', () => {
  compressedResults.forEach((f) => {
    if (f.compUrl) URL.revokeObjectURL(f.compUrl);
  });
  compressedResults.length = 0;
  queuedFiles.length = 0;

  fileList.innerHTML = '';
  queueList.innerHTML = '';
  resultsSection.classList.add('hidden');
  queueSection.classList.add('hidden');
  controlsSection.classList.add('hidden');

  updateSummaryStats();
});

// ── Helpers ───────────────────────────────────────────────────────────

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const val = bytes / Math.pow(k, i);
  // Round to whole numbers if >= 100 KB, or 1 decimal if < 100 KB / MB
  const decimals = (val >= 100 && i === 1) ? 0 : 1;
  return parseFloat(val.toFixed(decimals)) + ' ' + sizes[i];
}

function getMimeType(format) {
  const map = {
    JPEG: 'image/jpeg',
    PNG: 'image/png',
    WebP: 'image/webp',
  };
  return map[format] || 'application/octet-stream';
}

function getExtension(format) {
  const map = {
    JPEG: 'jpg',
    PNG: 'png',
    WebP: 'webp',
  };
  return map[format] || 'bin';
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ── Prevent Default Window Drag Over ──────────────────────────────────

window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault());

// ── Boot ──────────────────────────────────────────────────────────────

loadingOverlay.classList.remove('hidden');
initWorker();

/* _GIT_HISTORY_DUMMY_ */ /* Revision 26 - 0ltqtg */
