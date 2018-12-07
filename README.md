# cargo-coverage-annotations [![Build Status](https://api.travis-ci.org/orenbenkiki/cargo-coverage-annotations.svg?branch=master)](https://travis-ci.org/orenbenkiki/cargo-coverage-annotations) [![docs](https://docs.rs/cargo-coverage-annotations/badge.svg)](https://docs.rs/crate/cargo-coverage-annotations)

Ensure annotations in code match actual coverage.

## Installing

To install:

```
cargo install cargo-coverage-annotations
```

## Running

### Creating coverage XML file(s)

To run on a cargo project in the current working directory, first generate
`cobertura.xml` files(s) anywhere under the current working directory. This can
be done in one of _way_ too many ways, as there's no standard rust `cargo
coverage` for now.

Two options I have tested and you might want to consider are:

* `cargo tarpaulin --out Xml` will generate a single `cobertura.xml` file in the
  top level directory. This is much simpler to use than `kcov`, without
  requiring `cargo make`.

  Note that as of version 0.5.5, `tarpaulin` is still not 100% reliable. This
  might require you to insert spurious coverage annotations to the source code,
  which defeats their purpose.

* `cargo make coverage` will by default use `kcov` to generate several
  `cobertura.xml` files nested in the bowels of `target/coverage/...`. This
  requires installing `cargo make`, which I found to be more convenient than
  trying to create the magical incantations for running `kcov` myself.

  Note that `cargo make` version 0.7.11 insists all your files in the `tests`
  directory be named `test_*.rs`, and that there will be at least one such test
  file (in addition to any `#[test]` functions you might have in the sources).

  Note that `kcov`, as of version 34, also returns wrong coverage results, at
  least sometimes, at least for `rust`. It seems to be more robust than
  `tarpaulin`, though.

To combat the flakiness in the coverage reporting tools, reported coverage is
ignored for lines that contain only closing braces or only `else` statements, or
only comments. This seems to avoid the worst false coverage offenders; YMMV.

Of course, other tools generate other coverage file formats, and place them in
different places. If you look at https://codecov.io/bash you will see >1K lines
of code for detecting these files, and this doesn't cover the code for parsing
the different formats. So, if your favorite tool isn't supported, don't be
surprised, and pull requests are welcome ;-)

### Verifying the coverage annotations

To verify that the coverage annotations in the code match the actual coverage,
run `cargo coverage-annotations`. This will merge the coverage information from
all the `cobertura.xml` files, and compare the results with the coverage
annotation comments (see below).

### Integration with cargo make

If you use `cargo make`, here is one way to
integrate `cargo coverage-annotations` into your workflow:

```toml
[tasks.coverage-annotations]
description = "Verify the coverage annotations in the code"
category = "Test"
install_crate = "cargo-coverage-annotations"
command = "cargo"
args = ["coverage-annotations"

# Verify coverage annotations as part of `cargo make coverage-flow`
[tasks.post-coverage]
dependencies = [..., "coverage-annotations"]

# Verify coverage annotations as part of `cargo make build-flow`
# and `cargo make ci-flow`.
[tasks.pre-verify-project]
dependencies = [..., "coverage-flow"]
```

### Checking coverage annotations on a CI server

To keep your code base clean, it can be helpful to fail the CI build when the
code contains wrong coverage annotations. To achieve this, include `cargo
coverage-annotations`` in your CI build steps. For example, a minimal Travis
setup using `tarpaulin` might look like this:

```yaml
language: rust
cache: cargo
before_script:
- export PATH="$PATH:$HOME/.cargo/bin"
- which cargo-tarpaulin || cargo install cargo-tarpaulin
- which cargo-coverage-annotations || cargo install cargo-coverage-annotations
script:
- cargo build
- cargo test
- cargo tarpaulin --out Xml
- cargo coverage-annotations
```

Note that using `cache: cargo` is optional but highly recommended to speed up
the installation.

## Coverage annotations

Coverage annotations are comments that indicate the coverage status of the code
lines. By default, code lines are assumed to be covered by tests. Lines
that are not tested are expected to end with an explicit `// NOT TESTED` comment.
It is also possible to mark a line with a `// MAYBE TESTED` comment in
special cases (for example, lines that only execute on some platforms).

Sometimes a whole block of lines needs to be marked. In this case, it is
possible to surround such lines with `// BEGIN NOT TESTED` ... `// END NOT
TESTED` comments (or `// BEGIN MAYBE TESTED` ... `// END MAYBE TESTED`).
Inside such regions, it is possible to override the annotation for specific
lines with `// TESTED`, `// NOT TESTED` or `// MAYBE TESTED` comments.

Finally, some files might not be tested at all. In this case, they must contain
in one of their lines a `// FILE NOT TESTED` or `// FILE MAYBE TESTED` comment.
This includes examples files.

Coverage annotations are only used for files in the `src` directory and `tests`
directories. They ensure that when reading the code, one is aware of what is and
is not covered by the tests. Of course, line coverage is only the most basic
form of coverage tracking; that said, tracking it at each step is surprisingly
effective in isolating cases when the code does not behave as expected.

## License

`cargo-coverage-annotations` is distributed under the GNU General Public License
(Version 3.0). See the [LICENSE](LICENSE.txt) for details.
