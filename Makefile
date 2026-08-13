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
	cargo run --locked --bin meridian

bundle: verify-data
	cargo build --locked --release --bin meridian
	@case "$$(uname -s)" in \
		Linux) ./packaging/package-appimage && ./packaging/package-linux ;; \
		Darwin) cargo packager --release --formats dmg --out-dir dist ;; \
		MINGW*|MSYS*|CYGWIN*) cargo packager --release --formats nsis --out-dir dist ;; \
		*) echo "Unsupported packaging host: $$(uname -s)" >&2; exit 1 ;; \
	esac

test:
	cargo test --locked --all-targets

check: fmt lint test

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets --all-features -- -D warnings

clean:
	cargo clean
