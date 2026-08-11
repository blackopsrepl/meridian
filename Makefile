.PHONY: setup data-current data-cities verify-data run bundle test check fmt lint clean

setup:
	./tools/fetch-ephemeris --all
	./tools/fetch-geonames
	cargo build --locked

data-current:
	./tools/fetch-ephemeris --current

data-cities:
	./tools/fetch-geonames

verify-data:
	python3 tools/verify-release-data

run:
	cargo tauri dev

bundle: verify-data
	cargo tauri build

test:
	cargo test --locked --all-targets

check: fmt lint test

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets --all-features -- -D warnings

clean:
	cargo clean
