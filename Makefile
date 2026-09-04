.PHONY: fmt fmt-check lint test check all

all: fmt-check lint test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-features

# The crate must stay usable by a consumer that wants the types and nothing
# else — that is the whole point of the feature split.
check:
	cargo check --no-default-features
	cargo check --all-features
