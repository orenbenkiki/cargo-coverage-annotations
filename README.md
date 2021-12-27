# cargo-coverage-annotations

[![Verified](https://github.com/orenbenkiki/cargo-coverage-annotations/actions/workflows/on_push.yml/badge.svg)](https://github.com/orenbenkiki/cargo-coverage-annotations/actions/workflows/on_push.yml) [![Monthly audit](https://github.com/orenbenkiki/cargo-coverage-annotations/actions/workflows/monthly_audit.yml/badge.svg)](https://github.com/orenbenkiki/cargo-coverage-annotations/actions/workflows/on_updated_dependencies.yml) [![Api Docs](https://docs.rs/cargo-coverage-annotations/badge.svg)](https://docs.rs/crate/cargo-coverage-annotations)

Ensure annotations in code match actual coverage.

## Installing

To install:

```console
cargo install cargo-coverage-annotations
```

## Running

### Creating coverage XML file(s)

To run on a cargo project in the current working directory, first generate `cobertura.xml` files(s) anywhere under the
current working directory. There's no standard `cargo coverage`, so this code was tested against using `cargo tarpaulin
--out Xml`, and (a while back) using `cargo kcov` (which seems less actively maintained these days).

Of course, other tools generate other coverage file formats, and place them in different places. If you look at
[CodeCov](https://codecov.io/bash) you will see >1K lines of code for detecting these files, and this doesn't cover the
code for parsing the different formats. So, if your favorite tool isn't supported, pull requests are welcome ;-)

### Verifying the coverage annotations

To verify that the coverage annotations in the code match the actual coverage, run `cargo coverage-annotations`. This
will merge the coverage information from all the `cobertura.xml` files, and compare the results with the coverage
annotation comments (see below).

## Coverage annotations

Coverage annotations are comments that indicate the coverage status of the code lines. By default, code lines are
assumed to be covered by tests. Lines that are not tested are expected to end with an explicit `// NOT TESTED` comment.
It is also possible to mark a line with a `// MAYBE TESTED` comment in special cases (for example, lines that only
execute on some platforms). You can use `/* ... */` instead of `// ...` comments in you wish.

Sometimes a whole block of lines needs to be marked. In this case, it is possible to surround such lines with `// BEGIN
NOT TESTED` ... `// END NOT TESTED` comments (or `// BEGIN MAYBE TESTED` ... `// END MAYBE TESTED`). Inside such
regions, it is possible to override the annotation for specific lines with `// TESTED`, `// NOT TESTED` or `// MAYBE
TESTED` comments.

Some files might not be tested at all. In this case, they must contain in one of their lines a `// FILE NOT TESTED` or
`// FILE MAYBE TESTED` comment.

Sometimes code lines are actually tested but are marked as uncovered by the coverage tool (no tool is perfect). To
overcome this, you can mark a line (or a whole region) as `// APPEARS NOT TESTED`. This is treated exactly the same as
`// NOT TESTED` lines, but it helps the developer reading the code later on.

Coverage annotations are only used for files in the `src` directory and `tests` directories. They ensure that when
reading the code, one is aware of what is and is not covered by the tests. Of course, line coverage is only the most
basic form of coverage tracking; that said, tracking it at each step is surprisingly effective in isolating cases when
the code does not behave as expected.

## License

`cargo-coverage-annotations` is distributed under the GNU General Public License (Version 3.0). See the
[LICENSE](LICENSE.txt) for details.