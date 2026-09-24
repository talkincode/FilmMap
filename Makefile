.PHONY: check e2e book
check:
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test
e2e:
	cargo build
	bash tests/e2e/cli.sh target/debug/filmmap
	bash tests/e2e/scan.sh target/debug/filmmap tools
book:
	mdbook build
