.PHONY: build build-release build-bundler test test-native test-browser clean check lint

# Build for browser <script type="module"> usage (ESM, manual init)
build:
	wasm-pack build --target web --out-dir pkg

# Optimized release build (smaller .wasm via LTO + wasm-opt)
build-release:
	wasm-pack build --target web --release --out-dir pkg

# Build for bundler (webpack, Vite, etc.)
build-bundler:
	wasm-pack build --target bundler --out-dir pkg-bundler

# Run all tests: native cargo test + wasm-bindgen-test in headless browser
test: test-native test-browser

test-native:
	cargo test

test-browser:
	wasm-pack test --headless --chrome

# Cargo check (fast compilation check)
check:
	cargo check --target wasm32-unknown-unknown

# Lint with cargo clippy
lint:
	cargo clippy --target wasm32-unknown-unknown -- -D warnings

clean:
	cargo clean
	rm -rf pkg/ pkg-bundler/ pkg-node/
