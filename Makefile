PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: build test lint fmt-check install-local install-npm-local

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt-check:
	cargo fmt --check

install-local:
	cargo build --release
	mkdir -p "$(BINDIR)"
	cp target/release/flowleap "$(BINDIR)/flowleap"
	@echo "Installed flowleap to $(BINDIR)/flowleap"
	@echo "PATH entries named flowleap:"
	@which -a flowleap 2>/dev/null || true
	@echo "If your shell still runs an npm-installed wrapper, run: make install-npm-local && hash -r"

install-npm-local:
	cargo build --release
	@node -e 'const fs=require("fs"), path=require("path"), cp=require("child_process"); const entries=cp.execFileSync("which",["-a","flowleap"],{encoding:"utf8"}).trim().split(/\n+/).filter(Boolean); const targets=[]; for (const entry of entries) { const real=fs.realpathSync(entry); if (real.replaceAll("\\\\","/").includes("/node_modules/flowleap/bin/flowleap")) { const native=path.join(path.dirname(real), process.platform==="win32" ? "flowleap-native.exe" : "flowleap-native"); if (fs.existsSync(native)) targets.push(native); } } if (!targets.length) { console.error("No npm-installed flowleap-native found in PATH. Entries checked: "+entries.join(", ")); process.exit(1); } for (const target of [...new Set(targets)]) { fs.copyFileSync("target/release/flowleap", target); fs.chmodSync(target, 0o755); console.log("Updated npm flowleap-native at "+target); }'
