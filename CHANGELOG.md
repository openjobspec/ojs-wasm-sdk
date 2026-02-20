# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
