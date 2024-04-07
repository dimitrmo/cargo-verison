.PHONY: prod
prod:
	cargo build --bin cargo-verison
	ls -lah target/debug/cargo-verison
	cargo build --release --bin cargo-verison
	ls -lah target/release/cargo-verison

.PHONY: test
test:
	cargo test -- --test-threads 1
	cargo test --release -- --test-threads 1
