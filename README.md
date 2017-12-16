# cargo-coverage-annotations [![Build Status](https://api.travis-ci.org/orenbenkiki/cargo-coverage-annotations.svg?branch=master)](https://travis-ci.org/orenbenkiki/cargo-coverage-annotations)

Ensure annotations in code match actual coverage.

## Installing

To install:

```
cargo install cargo-coverage-annotations
```

## Running

To run on a cargo project in the current working directory:

```
cargo tarpaulin --out Xml
cargo coverage-annotations
```

This will generate a `cobertura.xml` file containing coverage information,
then verify that the annotations in the code match the actual coverage.

TODO: support additional coverage file formats (`kcov`, `gcov`).

## Checking coverage annotations on a CI server

To keep your code base clean, it can be helpful to fail the CI build when the
code contains wrong coverage annotations. To achieve this, include `cargo coverage-annotations``
in your CI build steps. For example, a minimal Travis setup might look like
this:

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
