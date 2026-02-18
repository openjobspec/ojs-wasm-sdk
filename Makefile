.PHONY: build test clean

build:
	wasm-pack build --target web

test:
	wasm-pack test --headless --chrome

clean:
	cargo clean
	rm -rf pkg/
