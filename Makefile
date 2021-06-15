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

.PHONY: integration-