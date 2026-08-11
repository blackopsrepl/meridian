.PHONY: setup data-current data-cities run test check fmt lint clean

setup:
	./tools/fetch-ephemeris --all
	./tools/fetch-geonames
	cargo build --locked

data-current:
	./tools/fetch-ephemeris --current

data-cities:
	./tools/fetch-geonames

run:
	cargo run --locked -- serve

test:
	cargo test --locked --all-targets

check: fmt lint test

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets --all-features -- -D warnings

clean:
	cargo clean
