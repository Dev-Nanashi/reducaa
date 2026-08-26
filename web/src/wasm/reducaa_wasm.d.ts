/* tslint:disable */
/* eslint-disable */

/**
 * Compress an image.
 *
 * # Arguments
 * * `data` — Raw image file bytes (JPEG, PNG, or WebP)
 * * `options` — JS object with optional fields: `quality`, `format`, `maxWidth`, `maxHeight`, `lossless`
 *
 * # Returns
 * A JS object with: `data` (Uint8Array), `format`, `originalSize`, `compressedSize`,
 * `width`, `height`, `reductionPercent`.
 */
export function compress(data: Uint8Array, options: any): any;

/**
 * Detect the image format from raw bytes.
 * Returns "jpeg", "png", "webp", or throws on unknown format.
 */
export function detectFormat(data: Uint8Array): string;

/**
 * Initialize the WASM module. Call once on load.
 */
export function initReducaa(): void;

/**
 * Inspect image headers to read format, orientation, and detect EXIF metadata.
 */
export function inspectImage(data: Uint8Array): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compress: (a: number, b: number, c: any) => [number, number, number];
    readonly detectFormat: (a: number, b: number) => [number, number, number, number];
    readonly initReducaa: () => void;
    readonly inspectImage: (a: number, b: number) => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
