SECRETCLI = docker exec -it secretdev /usr/bin/secretcli

.PHONY: all
all: clippy test

.PHONY: check
check:
	cargo check

.PHONY: check-receiver
check-receiver:
	$(MAKE) -C tests/example-receiver check

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: clippy-receiver
clippy-receiver:
	$(MAKE) -C tests/example-receiver clippy

.PHONY: test
test: unit-test unit-test-receiver integration-test

.PHONY: unit-test
unit-test:
	cargo test

.PHONY: unit-test-receiver
unit-test-receiver:
	$(MAKE) -C tests/example-receiver unit-test

.PHONY: integration-test
integration-test: compile-optimized compile-optimized-receiver
	tests/integration.sh

compile-optimized-receiver:
	$(MAKE) -C tests/example-receiver compile-optimized

.PHONY: list-code
list-code:
	$(SECRETCLI) query compute list-code

.PHONY: compile _compile
compile: _compile contract.wasm.gz
_compile:
	cargo build --target wasm32-unknown-unknown --locked
	cp ./target/wasm32-unknown-unknown/debug/*.wasm ./contract.wasm

.PHONY: compile-optimized _compile-optimized
compile-optimized: _compile-optimized contract.wasm.gz
_compile-optimized:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-u