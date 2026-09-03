# Bundle Size Budget

The 0.5.0 npm package preserves the advertised default feature set:

- generic browser and edge clients;
- Cloudflare, Deno, and Vercel clients;
- Service Worker and durable Background Sync support;
- AES-256-GCM encryption helpers; and
- JSON Schema validation.

The optimized default artifact is 370,278 raw bytes and 149,733 bytes with
deterministic gzip (`gzip -cn`). CI fails when the deterministic gzip size
exceeds 151,552 bytes (148 KiB), leaving 1,819 bytes or approximately 1.2% for
toolchain variance and requiring intentional review for larger growth.

## Why the former 50 KiB limit was replaced

The no-default core is already approximately 270 KB raw and 110 KB gzip before
Service Worker, encryption, schema, or runtime-specific edge exports are
enabled. The required default composition is therefore almost three times the
old 51,200-byte threshold before any optional feature can be removed.

The feature export matrix records raw and deterministic-gzip sizes for:

| Variant | Included surface | Raw bytes | Gzip bytes |
|---|---|---:|---:|
| `core` | Browser/generic-edge core only | 270,077 | 109,727 |
| `service-worker` | Core plus Service Worker and Background Sync | 304,669 | 123,117 |
| `encryption` | Core plus AES-256-GCM | 299,297 | 121,435 |
| `schema` | Core plus JSON Schema validation | 287,139 | 116,978 |
| `edge-cloudflare` | Core plus Cloudflare bindings | 283,603 | 115,302 |
| `edge-deno` | Core plus Deno bindings | 277,778 | 113,267 |
| `edge-vercel` | Core plus Vercel bindings | 278,838 | 113,665 |
| `edge-all` | Core plus all runtime-specific edge clients | 289,580 | 117,418 |
| `default` | All advertised default features | 370,278 | 149,733 |

Run `npm run test:features` to rebuild and print the complete matrix.

## Optimization evidence

All measurements below use the same 0.5.0 source and advertised default
features:

| Configuration | Raw bytes | Gzip bytes |
|---|---:|---:|
| Rust `opt-level = "z"` + fat LTO + converged `wasm-opt -Oz` | 370,278 | 149,733 |
| Additional `wasm-opt -Os --converge` pass | 370,286 | 149,731 |
| Additional `wasm-opt -O4 --converge` pass | 381,678 | 152,041 |
| Rust `opt-level = "s"` before `wasm-opt -Oz` | 385,321 | 153,252 |
| Rust `opt-level = 3` before `wasm-opt -Oz` | 423,925 | 167,925 |

The two-byte result from a second `-Os` pass is not a meaningful improvement
and adds another optimizer pass to every release. Strip/vacuum/closed-world
experiments also increased the artifact. The existing `z`/fat-LTO/one-codegen
unit/abort/strip/converged-`-Oz` profile remains the smallest safe configuration
tested.

`twiggy top` reports 1,818 retained rows with no single removable subsystem;
the previous named pre-bindgen profile attributed roughly 77 KB raw to 62
public async `future_to_promise` wrappers. Removing those wrappers, changing
the default feature composition, or splitting runtime surfaces would alter the
advertised API/package behavior and is outside a mechanical release fix.
