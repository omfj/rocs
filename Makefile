.PHONY: help build install clean test

help:
	@echo "   _ __ ___   ___ ___  "
	@echo "  | '__/ _ \ / __/ __| "
	@echo "  | | | (_) | (__\__ \ "
	@echo "  |_|  \___/ \___|___/ "
	@echo ""
	@echo "          rocs         "
	@echo ""
	@echo "Usage: make [command]"
	@echo ""
	@echo "Command:"
	@echo "  build     Build the project"
	@echo "  install   Install the project"
	@echo "  clean     Clean the build artifacts"

build:
	@cargo build --release

install:
	@cargo install --path .

clean:
	@cargo clean

test:
	@cargo test
