.PHONY: fmt lint test ci build-slack push-slack build-ollama push-ollama ref

fmt:
	cargo fmt --all

lint:
	cargo fmt --all --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

ci: lint test

# Version is read from Cargo.toml — edit version there and in grimoire.json.
SLACK_VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' plugins/slack/Cargo.toml)
SLACK_IMAGE := ghcr.io/kakilangit/grimoire-slack

build-slack:
	docker build -t $(SLACK_IMAGE):$(SLACK_VERSION) -t $(SLACK_IMAGE):latest -f plugins/slack/Dockerfile .

push-slack: build-slack
	docker push $(SLACK_IMAGE):$(SLACK_VERSION)
	docker push $(SLACK_IMAGE):latest

OLLAMA_VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' plugins/ollama/Cargo.toml)
OLLAMA_IMAGE := ghcr.io/kakilangit/grimoire-ollama

build-ollama:
	docker build -t $(OLLAMA_IMAGE):$(OLLAMA_VERSION) -t $(OLLAMA_IMAGE):latest -f plugins/ollama/Dockerfile .

push-ollama: build-ollama
	docker push $(OLLAMA_IMAGE):$(OLLAMA_VERSION)
	docker push $(OLLAMA_IMAGE):latest

# Generate a plugin ref from an image path: make ref IMAGE=ghcr.io/kakilangit/grimoire-slack
ref:
	@printf '%s' "$(IMAGE)" | shasum -a 256 | cut -c1-12
