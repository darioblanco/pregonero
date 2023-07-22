.DEFAULT_GOAL := help
SHELL := /usr/bin/env bash
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-builtin-variables

COMPOSE := $(shell command -v podman-compose 2> /dev/null || command -v docker-compose 2> /dev/null)

##########
# Global #
##########
.PHONY: help init install

help: ## list available commands
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

format: init ## format the code
	cargo fmt

init: ## verify that all the required commands are already installed
	@if [ -z "$$CI" ]; then \
		function cmd { \
			if ! command -v "$$1" &>/dev/null ; then \
				echo "error: missing required command in PATH: $$1" >&2 ;\
				return 1 ;\
			fi \
		} ;\
		cmd cargo; \
		cp .githooks/* .git/hooks/ ;\
	fi

install: init ## install cargo tools
	cargo install cargo-tarpaulin cargo-watch grcov


#######
# App #
#######

.PHONY: api-build api-coverage

build: init ## build the app
	cargo build --release

coverage: init ## test the app with coverage enabled
	cargo tarpaulin --exclude-files src/main.rs

start: init redis-start build ## start the app
	RUST_LOG=info ./target/release/pregonero

test: init ## test the app
	cargo test

#############
# Container #
#############

.PHONY: redis-attach redis-start

redis-attach: init ## attach to the redis container
	$(COMPOSE) exec redis sh

redis-start: init ## start and attach to the redis container
	$(COMPOSE) up -d redis
