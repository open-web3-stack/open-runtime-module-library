check: githooks
	./scripts/run.sh check --no-default-features --target=wasm32-unknown-unknown

check-tests: githooks
	./scripts/run.sh check --tests

test: githooks
	./scripts/run.sh test
	cargo test --manifest-path nft/Cargo.toml -p orml-nft --features disable-tokens-by-owner

GITHOOKS_SRC = $(wildcard githooks/*)
GITHOOKS_DEST = $(patsubst githooks/%, $(GITHOOK)/%, $(GITHOOKS_SRC))

GITHOOK := $(shell git rev-parse --git-path hooks)

$(GITHOOK):
	mkdir $(GITHOOK)

$(GITHOOK)/%: githooks/%
	cp "$^" "$(GITHOOK)"

githooks: $(GITHOOK) $(GITHOOKS_DEST)

init: githooks

format:
	./scripts/run.sh "fmt"


# Standalone development workflow targets
# Running those inside existing workspace will break due to Cargo unable to support nested worksapce

Cargo.toml: Cargo.dev.toml
	cp Cargo.dev.toml Cargo.toml

dev-format: Cargo.toml
	cargo fmt --all

dev-format-check: Cargo.toml
	cargo fmt --all -- --check

# needs to use run.sh to check individual projects because
#   --no-default-features is not allowed in the root of a virtual workspace
dev-check: Cargo.toml check

dev-check-tests: Cargo.toml
	cargo check --tests --all

dev-test: Cargo.toml
	cargo test --all
	cargo test --manifest-path nft/Cargo.toml -p orml-nft --features disable-tokens-by-owner
