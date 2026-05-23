.PHONY: fmt lint test ci build push ref

PLUGINS := $(notdir $(wildcard plugins/*))
REGISTRY := ghcr.io/kakilangit

fmt:
	cargo fmt --all

lint:
	cargo fmt --all --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

ci: lint test

# ── Plugin build/push ────────────────────────────────────────────────────────
# Usage: make build PLUGIN=ollama
#        make push  PLUGIN=slack
#        make build (builds all plugins)

plugin-version = $(shell sed -n 's/^version = "\(.*\)"/\1/p' plugins/$(1)/Cargo.toml)
plugin-image   = $(REGISTRY)/grimoire-$(1)

ifdef PLUGIN
build:
	docker build \
		-t $(call plugin-image,$(PLUGIN)):$(call plugin-version,$(PLUGIN)) \
		-t $(call plugin-image,$(PLUGIN)):latest \
		-f plugins/$(PLUGIN)/Dockerfile .

push: build
	docker push $(call plugin-image,$(PLUGIN)):$(call plugin-version,$(PLUGIN))
	docker push $(call plugin-image,$(PLUGIN)):latest
else
build:
	@$(foreach p,$(PLUGINS),\
		echo "=== building $(p) ===" && \
		docker build \
			-t $(call plugin-image,$(p)):$(call plugin-version,$(p)) \
			-t $(call plugin-image,$(p)):latest \
			-f plugins/$(p)/Dockerfile . && \
	) true

push:
	@$(foreach p,$(PLUGINS),\
		$(MAKE) push PLUGIN=$(p) && \
	) true
endif

# Generate a plugin ref: make ref IMAGE=ghcr.io/kakilangit/grimoire-slack
ref:
	@printf '%s' "$(IMAGE)" | shasum -a 256 | cut -c1-12
