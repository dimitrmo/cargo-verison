.PHONY: prod
prod:
	cargo build --release --bin cargo-verison --all-features
	ls -lah target/release/cargo-verison

.PHONY: dev
dev:
	cargo build --bin cargo-verison --all-features
	ls -lah target/debug/cargo-verison

.PHONY: test
test:
	cargo test -- --test-threads 1
	cargo test --release -- --test-threads 1
