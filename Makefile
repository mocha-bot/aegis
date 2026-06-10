.PHONY: build test lint fmt release clean

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy

fmt:
	cargo fmt

release:
	@test -n "$(VERSION)" || (echo "Usage: make release VERSION=0.1.0" && exit 1)
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	git push origin "v$(VERSION)"
	@echo "Tag pushed. GitHub Actions will build and publish:"
	@echo "  https://github.com/mocha-bot/aegis/releases"

clean:
	cargo clean
	rm -rf benchmark/target
