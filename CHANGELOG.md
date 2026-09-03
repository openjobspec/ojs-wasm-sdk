# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-09-02

### Added
- Durable IndexedDB-backed Service Worker Background Sync with concurrent leases
- Cargo feature gates for edge runtimes, Service Worker support, encryption, and schema validation
- npm pack/install/import/TypeScript/browser smoke validation and an MSRV 1.75 CI job

### Fixed
- npm 0.4.0 entry-point, export, type, WASM, and generated-snippet packaging
- Release builds remove stale generated output before producing versioned npm artifacts
- Replaced the unattainable 50 KiB limit with a measured 151,552-byte deterministic-gzip regression budget after profiling the required feature set
- Unicode-safe encryption key parsing and fallible entropy propagation
- JSON Schema integer handling for zero-fraction number representations
- Cargo 1.75-compatible dependency resolution and v3 lockfile

### Changed
- Release builds now use `opt-level = "z"`, aborting panics, symbol stripping, one codegen unit, and converged `wasm-opt -Oz`
- Default features preserve the established full 0.4.0 JavaScript/WASM ABI
- CI actions are immutable, wasm-pack installation is version-locked, and release artifacts gain SBOM, checksum, and provenance attestations
- npm publish dry-runs consume the exact local tarball already validated by the package smoke test

## [0.4.0] - 2026-04-20

### Added
- `enqueue_with_options()` method on all clients for queue/priority/timeout/delay/tags support
- Workflow support: `chain()`, `group()`, `batch()` builder functions
- `workflow()` and `get_workflow()` methods on all clients
- `workflow` module (`src/workflow.rs`) with wasm-bindgen exported builder functions
- `WorkflowResponse`, `WorkflowState`, `WorkflowMetadata` types
- TypeScript type definitions (`ojs-wasm-sdk.d.ts`)
- `package.json` for npm publishing as `@openjobspec/wasm`
- Example files: browser HTML demo, Cloudflare Worker, Deno Deploy
- Expanded test suite covering workflow builders and edge client construction
- Additional Makefile targets: `build-release`, `build-bundler`, `check`, `lint`

### Fixed
- `EnqueueRequest` construction in `OJSClient` and `ServiceWorkerClient` now includes `options` field
- Batch enqueue now propagates per-job options from JS input

### Changed
- `EnqueueOptions` now derives `Deserialize` (required for `enqueue_with_options`)
- README rewritten with comprehensive API reference, workflow docs, edge runtime examples, and limitations section
