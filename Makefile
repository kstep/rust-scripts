
build:
	cargo build

release: test
	cargo build --release

test:
	cargo test

clean:
	cargo clean

install: release
	find ./target/release -maxdepth 1 -type f -executable -exec \
		install -D -v -s -o root -g root -m 0755 -t /usr/local/bin {} +

.PHONY: build test clean release install
