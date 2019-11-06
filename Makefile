check: githooks
	./scripts/run.sh check --no-default-features

check-tests: githooks
	./scripts/run.sh check --tests

test: githooks
	./scripts/run.sh test

GITHOOKS_SRC = $(wildcard githooks/*)
GITHOOKS_DEST = $(patsubst githooks/%, $(GITHOOK)/%, $(GITHOOKS_SRC))

GITHOOK := $(shell git rev-parse --git-path hooks)

$(GITHOOK):
	mkdir $(GITHOOK)

$(GITHOOK)/%: githooks/%
	cp "$^" "$(GITHOOK)"

githooks: $(GITHOOK) $(GITHOOKS_DEST)

init: githooks
