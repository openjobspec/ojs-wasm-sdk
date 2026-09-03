import { access, rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const directory = resolve(process.cwd(), process.argv[2] ?? 'pkg');

await Promise.all([
  access(resolve(directory, 'ojs_wasm_sdk.js')),
  access(resolve(directory, 'ojs_wasm_sdk.d.ts')),
  access(resolve(directory, 'ojs_wasm_sdk_bg.wasm')),
]);

// wasm-pack writes a catch-all .gitignore into its output. npm honors nested
// ignore files while packing the root package, which would otherwise omit the
// generated inline-JS snippets imported by ojs_wasm_sdk.js.
await rm(resolve(directory, '.gitignore'), { force: true });
