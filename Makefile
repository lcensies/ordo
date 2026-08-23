# ordo — build + observability PoCs
#
# Usage:
#   make build               # cargo build --release
#   make test                # unit tests
#   make poc                 # both PoC scenarios against $(TARGET)
#   make poc-exec            # baseline curl+run vs ordo send --exec
#   make poc-download        # baseline curl vs ordo send (transfer only)
#   make poc-local           # both scenarios on localhost (needs sudo docker)
#
# Knobs:
#   TARGET=dev-vm-2          # ssh alias (or: local)
#   ARGS="--query curl,wget" # extra flags passed through to poc.sh

TARGET ?= dev-vm-2
ARGS   ?=

.PHONY: build test poc poc-exec poc-download poc-local clean

build:
	cargo build --release

test:
	cargo test --release

poc: poc-download poc-exec

poc-exec: build
	bash observability/poc.sh --target $(TARGET) --scenario exec $(ARGS)

poc-download: build
	bash observability/poc.sh --target $(TARGET) --scenario download $(ARGS)

poc-local:
	$(MAKE) poc TARGET=local

clean:
	cargo clean
