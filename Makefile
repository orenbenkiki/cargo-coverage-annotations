ALL_SOURCES = $(shell git ls-files .)

RS_SOURCES = $(filter %.rs, $(ALL_SOURCES))

TOML_SOURCES = $(filter %.toml, $(ALL_SOURCES))

CARGO_SOURCES = $(RS_SOURCES) $(TOML_SOURCES)

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([a-zA-Z_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		print("%-20s %s" % (target, help.replace('TODO-', 'TODO')))
endef
export PRINT_HELP_PYSCRIPT

help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

TAGS: $(RS_SOURCES)  ## TAGS file for vim or Emacs.
	rust-ctags .

TEST_FLAGS = RUST_TEST_THREADS=1 RUST_BACKTRACE=1

retest:  ## force re-run tests
	$(TEST_FLAGS) ./with_configuration.sh base cargo test -- --nocapture

test: .make.test  ## run tests

.make.test: $(CARGO_SOURCES)
	$(TEST_FLAGS) ./with_configuration.sh base cargo test -- --nocapture
	touch $@

check: .make.check  ## check the sources

.make.check: $(CARGO_SOURCES)
	./with_configuration.sh check cargo check --tests
	touch $@

build: .make.build  ## build the binaries

.make.build: $(CARGO_SOURCES)
	$(TEST_FLAGS) ./with_configuration.sh base cargo build
	$(TEST_FLAGS) ./with_configuration.sh base cargo test --no-run
	touch $@

TODO = todo$()x

pc: $(TODO) fmt staged clippy test doc outdated audit  ## check everything before commit

ci: $(TODO) fmt clippy doc outdated audit  ## check everything in a CI server

fmt: .make.fmt  ## check code format
	
.make.fmt: $(CARGO_SOURCES)
	cargo fmt -- --check
	touch $@

refmt: .make.refmt  ## reformat code
	
.make.refmt: $(CARGO_SOURCES)
	cargo fmt
	touch $@

staged:  ## check everything is staged for git commit
	@if git status . | grep -q 'Changes not staged\|Untracked files'; then git status; false; else true; fi

$(TODO): .make.$(TODO)  ## check there are no leftover TODO-X
	
.make.$(TODO): $(ALL_SOURCES)
	cargo $(TODO)
	touch $@

outdated: .make.outdated  ## check all dependencies are up-to-date
	
.make.outdated: $(TOML_SOURCES)
	cargo outdated --root-deps-only --exit-code 1
	touch $@

clippy: .make.clippy  ## check for code smells with clippy
	
.make.clippy: .make.check
	./with_configuration.sh check cargo clippy -- --no-deps
	touch $@

doc: .make.doc  ## generate documentation
	
.make.doc: $(ALL_SOURCES)
	./with_configuration.sh base cargo doc --no-deps
	touch $@

coverage: .make.coverage  ## generate coverage report

.make.coverage: $(CARGO_SOURCES)
	$(TEST_FLAGS) ./with_configuration.sh tarpaulin cargo tarpaulin --out Xml
	touch $@

audit: .make.audit  ## audit dependencies for bugs or security issues
	
.make.audit: $(TOML_SOURCES)
	cargo audit
	touch $@

clean:  ## remove all build, test, coverage and Python artifacts
	rm -f .make.*
	rm -rf target .target.*

pp: pc  ## pre-publish check
	./with_configuration.sh publish cargo publish --dry-run

publish: pp  ## actually publish
	./with_configuration.sh publish cargo publish
