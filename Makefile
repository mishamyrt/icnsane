VERSION = 0.0.0
.DEFAULT_GOAL := build

.PHONY: all build clean lint fmt test check publish

build:
	@cargo build --profile dist

clean:
	@cargo clean

lint:
	@cargo clippy --all

fmt:
	@cargo fmt --all

test:
	@cargo test

check: test
	@cargo fmt --all --check
	@cargo clippy --all

publish:
	@sed -E 's/^version = "[^"]+"/version = "${VERSION}"/' Cargo.toml > Cargo.toml.tmp
	@mv Cargo.toml.tmp Cargo.toml
	@cargo update -p icnsane
	@git add Makefile Cargo.toml Cargo.lock
	@git commit -m "chore: release ${VERSION} 🔥"
	@git tag "v${VERSION}"
	@git-cliff -o CHANGELOG.md
	@git tag -d "v${VERSION}"
	@git add CHANGELOG.md
	@git commit --amend --no-edit
	@git tag -a "v${VERSION}" -m "release v${VERSION}"
	@git push
	@git push --tags
