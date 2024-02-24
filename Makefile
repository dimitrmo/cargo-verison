.PHONY: prod
prod:
	cargo build --bin cargo-verison
	cargo build --release --bin cargo-verison

.PHONY: test
test:
	cargo test
