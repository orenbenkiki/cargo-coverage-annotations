# cargo-coverage-annotations [![Build Status](https://api.travis-ci.org/orenbenkiki/cargo-coverage-annotations.svg?branch=master)](https://travis-ci.org/orenbenkiki/cargo-coverage-annotations)

Ensure annotations in code match actual coverage.

## Installing

To install:

```
cargo install cargo-coverage-annotations
```

## Running

### Creating a coverage XML files

To run on a cargo project in the current working directory, first generate a
`cobertura.xml` file anywhere under the current working directory. This can be
done in one of _way_ too many ways, as there's no standard rust `cargo coverage`
for now.

Two options I have tested and you might want to consider are:

* `cargo tarpaulin --out Xml` will generate the `cobertura.xml` file in the top
  level directory. This is much simpler to use than `kcov`, without requiring
  `cargo make` and strange heuristics for selecting the `cobertura.xml` file.

  However, as of version 0.5.5, `tarpaulin` is still buggy. For example, it
  tends to complain `} else {` lines are not covered, even though both branches
  of the `if` statement are covered. This will require you to insert spurious
  coverage annotations to the source code, which defeats their purpose.

* `cargo make coverage` will by default use `kcov` to generate a `cobertura.xml`
  file nested in the bowels of `target/coverage/...`. This requires installing
  `cargo make`, which I found to be more convenient than trying to create the
  magical incantations for running `kcov` myself. TODO: Create a sample
  `Makefile.toml` that automates running the coverage and then verifying the
  annotations, in a single `cargo make` command.

  Note that `cargo make` version 0.7.11 insists all your files in the `tests`
  directory be named `test_*.rs`, and that there will be at least one such test
  file (in addition to any `#[test]` functions you might have in the sources).

  Note that `kcov` also returns wrong coverage results, at least sometimes, at
  least for `rust`, at least for now. It seems like it isn't as bad as
  `tarpaulin`, but you'll still need to add some spurious coverage annotations
  to the source.

  At any rate, `kcov` creates several `cobertura.xml` files. To choose between
  multiple candidate files, `cargo-coverage-annotations` will select the one
  whose path contains the word `merged`. This is admittedly a hack.

Of course, other tools generate other coverage file formats, and place them in
different places. If you look at https://codecov.io/bash you will see >1K lines
of code for detecting these files, and this doesn't cover the code for parsing
the different formats. So, if your favorite tool isn't supported, don't be
surprised, and pull requests are welcome ;-)

### Verifying the coverage annotations

To verify that the coverage annotations in the code match the actual
coverage, run `cargo coverage-annotations`.

## Checking coverage annotations on a CI server

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

Coverage annotations are only used for files in the `src` directory. They ensure
that when reading the code, one is aware of what is and is not covered by tests.
Of course, line coverage is only the most basic form of coverage tracking; that
said, tracking it at each step is surprisingly effective in isolating cases when
the code does not behave as expected.

## License

`cargo-coverage-annotations` is distributed under the GNU General Public License
(Version 3.0). See the [LICENSE](LICENSE.txt) for details.
