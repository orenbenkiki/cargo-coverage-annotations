// Copyright (C) 2017-2021 Oren Ben-Kiki <oren@ben-kiki.org>
//
// This file is part of cargo-coverage-annotations.
//
// cargo-coverage-annotations is free software: you can redistribute it and/or
// modify it under the terms of the GNU General Public License, version 3, as
// published by the Free Software Foundation.
//
// cargo-coverage-annotations is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
// Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// cargo-coverage-annotations. If not, see <http://www.gnu.org/licenses/>.

#![doc = include_str!("../README.md")]
#![deny(warnings)]
#![deny(rust_2018_idioms)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::vec::Vec;
use xml::reader::{EventReader, XmlEvent};

/// The current crate version: 0.4.3
const VERSION: &str = "0.4.3";

#[doc(hidden)]
enum LineMark {
    None,
    LineTested,
    LineMaybeTested,
    LineNotTested,
    LineFlakyTested,
    BeginMaybeTested,
    BeginNotTested,
    BeginFlakyTested,
    EndMaybeTested,
    EndNotTested,
    EndFlakyTested,
    FileMaybeTested,
    FileNotTested,
    FileFlakyTested,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
enum LineAnnotation {
    Tested(bool),
    MaybeTested(bool),
    NotTested(bool),
    FlakyTested(bool),
}

#[doc(hidden)]
const fn is_explicit(line_annotation: LineAnnotation) -> bool {
    matches!(
        line_annotation,
        LineAnnotation::Tested(true)
            | LineAnnotation::MaybeTested(true)
            | LineAnnotation::NotTested(true)
            | LineAnnotation::FlakyTested(true)
    )
}

#[doc(hidden)]
#[derive(Debug)]
enum FileAnnotations {
    LineAnnotations(Vec<LineAnnotation>),
    MaybeTested,
    NotTested,
}

#[doc(hidden)]
fn main() {
    let flaky_policy = process_args();

    let mut coverage_annotations = HashMap::new();
    let mut source_annotations = HashMap::new();
    collect_dir_annotations(
        flaky_policy,
        Path::new("."),
        &mut source_annotations,
        &mut coverage_annotations,
    )
    .unwrap();
    let exit_status =
        report_wrong_annotations(flaky_policy, &coverage_annotations, &source_annotations);
    std::process::exit(exit_status);
}

#[doc(hidden)]
fn collect_dir_annotations(
    flaky_policy: FlakyPolicy,
    dir: &Path,
    source_annotations: &mut HashMap<String, FileAnnotations>,
    coverage_annotations: &mut HashMap<String, HashMap<i32, bool>>,
) -> std::io::Result<()> {
    let entries: fs::ReadDir = fs::read_dir(dir)?;
    for entry in entries {
        let entry: fs::DirEntry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_annotations(
                flaky_policy,
                &path,
                source_annotations,
                coverage_annotations,
            )?;
        } else if let Ok(canonical) = fs::canonicalize(path) {
            let file_name = canonical.as_path().to_str().unwrap();
            if file_name.ends_with("/cobertura.xml") {
                collect_coverage_annotations(canonical.as_path(), coverage_annotations);
            } else if file_name.ends_with(".rs") {
                let annotations = collect_file_annotations(flaky_policy, canonical.as_path());
                source_annotations.insert(file_name.to_string(), annotations);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
#[doc(hidden)]
fn collect_file_annotations(flaky_policy: FlakyPolicy, path: &Path) -> FileAnnotations {
    let file = File::open(path).unwrap_or_else(|_| panic!("can't open {}", path.to_str().unwrap()));
    let file = BufReader::new(file);
    let mut region_annotation = LineAnnotation::Tested(false);
    let mut is_file_not_tested = false;
    let mut is_file_maybe_tested = false;
    let mut is_file_flaky_tested = false;
    let mut line_annotations = Vec::new();
    let untrusted_regex = Regex::new(
        r"(?x)
            ^
            \s*
            (?:
                    \}
                    \s*
                    (?:
                        \)\s*
                    )*
                    (?:
                        ;\s*
                    )
                |
                    (?:
                        \}\s*
                    )?
                    else
                    \s*
                    (?:
                        \{\s*
                    )?
                |
                    \#
                    !?
                    \[.*\]\s*
                |
                    impl
                    [\s<]
                    .*
            )?
            (?:
                /[/*].*
            )?
            $
        ",
    )
    .unwrap();
    for (mut line_number, line) in file.lines().enumerate() {
        line_number += 1;
        let line_text = line.unwrap();
        let line_mark = extract_line_mark(path.to_str().unwrap(), line_number, line_text.as_ref());
        let (line_annotation, next_region_annotation) = match (line_mark, region_annotation) {
            (LineMark::None, region_annotation) => {
                if line_text.contains("unreachable!()") {
                    (LineAnnotation::NotTested(false), region_annotation)
                } else {
                    (region_annotation, region_annotation)
                }
            }

            (LineMark::LineTested, LineAnnotation::Tested(_)) => {
                eprintln!(
                    "{}:{}: redundant TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (LineAnnotation::Tested(true), LineAnnotation::Tested(false))
            }
            (LineMark::LineTested, region_annotation) => {
                (LineAnnotation::Tested(true), region_annotation)
            }

            (LineMark::LineNotTested, LineAnnotation::NotTested(_)) => {
                eprintln!(
                    "{}:{}: redundant NOT TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (
                    LineAnnotation::NotTested(true),
                    LineAnnotation::NotTested(false),
                )
            }
            (LineMark::LineNotTested, region_annotation) => {
                (LineAnnotation::NotTested(true), region_annotation)
            }

            (LineMark::LineMaybeTested, LineAnnotation::MaybeTested(_)) => {
                eprintln!(
                    "{}:{}: redundant MAYBE TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (
                    LineAnnotation::MaybeTested(true),
                    LineAnnotation::MaybeTested(false),
                )
            }
            (LineMark::LineMaybeTested, region_annotation) => {
                (LineAnnotation::MaybeTested(true), region_annotation)
            }

            (LineMark::LineFlakyTested, LineAnnotation::FlakyTested(_)) => {
                eprintln!(
                    "{}:{}: redundant FLAKY TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (
                    LineAnnotation::FlakyTested(true),
                    LineAnnotation::FlakyTested(false),
                )
            }
            (LineMark::LineFlakyTested, region_annotation) => {
                (LineAnnotation::FlakyTested(true), region_annotation)
            }

            (LineMark::BeginNotTested, LineAnnotation::Tested(_)) => (
                LineAnnotation::NotTested(false),
                LineAnnotation::NotTested(false),
            ),
            (LineMark::BeginNotTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested BEGIN NOT TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::BeginMaybeTested, LineAnnotation::Tested(_)) => (
                LineAnnotation::MaybeTested(false),
                LineAnnotation::MaybeTested(false),
            ),
            (LineMark::BeginMaybeTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested BEGIN MAYBE TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::BeginFlakyTested, LineAnnotation::Tested(_)) => (
                LineAnnotation::FlakyTested(false),
                LineAnnotation::FlakyTested(false),
            ),
            (LineMark::BeginFlakyTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested BEGIN FLAKY TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::EndNotTested, LineAnnotation::NotTested(_)) => (
                LineAnnotation::NotTested(false),
                LineAnnotation::Tested(false),
            ),
            (LineMark::EndNotTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested END NOT TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::EndMaybeTested, LineAnnotation::MaybeTested(_)) => (
                LineAnnotation::MaybeTested(false),
                LineAnnotation::Tested(false),
            ),
            (LineMark::EndMaybeTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested END MAYBE TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::EndFlakyTested, LineAnnotation::FlakyTested(_)) => (
                LineAnnotation::FlakyTested(false),
                LineAnnotation::Tested(false),
            ),
            (LineMark::EndFlakyTested, region_annotation) => {
                eprintln!(
                    "{}:{}: ignored nested END FLAKY TESTED coverage annotation",
                    path.to_str().unwrap(),
                    line_number
                );
                (region_annotation, region_annotation)
            }

            (LineMark::FileNotTested, region_annotation) => {
                if is_file_not_tested || is_file_maybe_tested || is_file_flaky_tested {
                    eprintln!(
                        "{}:{}: repeated FILE NOT/MAYBE/FLAKY TESTED coverage annotation",
                        path.to_str().unwrap(),
                        line_number
                    );
                }
                is_file_not_tested = true;
                (region_annotation, region_annotation)
            }

            (LineMark::FileMaybeTested, region_annotation) => {
                if is_file_not_tested || is_file_maybe_tested || is_file_flaky_tested {
                    eprintln!(
                        "{}:{}: repeated FILE NOT/MAYBE/FLAKY TESTED coverage annotation",
                        path.to_str().unwrap(),
                        line_number
                    );
                }
                is_file_maybe_tested = true;
                (region_annotation, region_annotation)
            }

            (LineMark::FileFlakyTested, region_annotation) => {
                if is_file_not_tested || is_file_maybe_tested || is_file_flaky_tested {
                    eprintln!(
                        "{}:{}: repeated FILE NOT/MAYBE/FLAKY TESTED coverage annotation",
                        path.to_str().unwrap(),
                        line_number
                    );
                }
                is_file_flaky_tested = true;
                (region_annotation, region_annotation)
            }
        };
        line_annotations.push(if untrusted_regex.is_match(line_text.as_ref()) {
            LineAnnotation::MaybeTested(false)
        } else {
            line_annotation
        });
        region_annotation = next_region_annotation;
    }
    if is_file_maybe_tested || (is_file_flaky_tested && flaky_policy == FlakyPolicy::MaybeTested) {
        verify_untested_file_annotations(path, &line_annotations);
        FileAnnotations::MaybeTested
    } else if is_file_not_tested || (is_file_flaky_tested && flaky_policy == FlakyPolicy::NotTested)
    {
        verify_untested_file_annotations(path, &line_annotations);
        FileAnnotations::NotTested
    } else {
        FileAnnotations::LineAnnotations(line_annotations)
    }
}

#[doc(hidden)]
fn verify_untested_file_annotations(path: &Path, line_annotations: &[LineAnnotation]) {
    for (mut line_number, line_annotation) in line_annotations.iter().enumerate() {
        line_number += 1;
        if is_explicit(*line_annotation) {
            eprintln!(
                "{}:{}: line coverage annotation in a FILE which is NOT/MAYBE/FLAKY TESTED",
                path.to_str().unwrap(),
                line_number
            );
        }
    }
}

#[doc(hidden)]
fn extract_line_mark(path: &str, line_number: usize, line: &str) -> LineMark {
    if line.contains("// APPEARS NOT TESTED")
        || line.contains("/* APPEARS NOT TESTED")
        || line.contains("// BEGIN APPEARS NOT TESTED")
        || line.contains("/* BEGIN APPEARS NOT TESTED")
        || line.contains("// END APPEARS NOT TESTED")
        || line.contains("/* END APPEARS NOT TESTED")
        || line.contains("// FILE APPEARS NOT TESTED")
        || line.contains("/* FILE APPEARS NOT TESTED")
    {
        eprintln!(
            "{}:{}: obsolete APPEARS TESTED directive, use FLAKY TESTED instead",
            path, line_number
        );
        LineMark::None
    } else if line.contains("// TESTED") || line.contains("/* TESTED") {
        LineMark::LineTested
    } else if line.contains("// MAYBE TESTED") || line.contains("/* MAYBE TESTED") {
        LineMark::LineMaybeTested
    } else if line.contains("// NOT TESTED") || line.contains("/* NOT TESTED") {
        LineMark::LineNotTested
    } else if line.contains("// FLAKY TESTED") || line.contains("/* FLAKY TESTED") {
        LineMark::LineFlakyTested
    } else if line.contains("// BEGIN MAYBE TESTED") || line.contains("/* BEGIN MAYBE TESTED") {
        LineMark::BeginMaybeTested
    } else if line.contains("// BEGIN NOT TESTED") || line.contains("/* BEGIN NOT TESTED") {
        LineMark::BeginNotTested
    } else if line.contains("// BEGIN FLAKY TESTED") || line.contains("/* BEGIN FLAKY TESTED") {
        LineMark::BeginFlakyTested
    } else if line.contains("// END MAYBE TESTED") || line.contains("/* END MAYBE TESTED") {
        LineMark::EndMaybeTested
    } else if line.contains("// END NOT TESTED") || line.contains("/* END NOT TESTED") {
        LineMark::EndNotTested
    } else if line.contains("// END FLAKY TESTED") || line.contains("/* END FLAKY TESTED") {
        LineMark::EndFlakyTested
    } else if line.contains("// FILE MAYBE TESTED") || line.contains("// FILE MAYBE TESTED") {
        LineMark::FileMaybeTested
    } else if line.contains("// FILE NOT TESTED") || line.contains("/* FILE NOT TESTED") {
        LineMark::FileNotTested
    } else if line.contains("// FILE FLAKY TESTED") || line.contains("/* FILE FLAKY TESTED") {
        LineMark::FileFlakyTested
    } else {
        LineMark::None
    }
}

#[doc(hidden)]
fn collect_coverage_annotations(
    path: &Path,
    coverage_annotations: &mut HashMap<String, HashMap<i32, bool>>,
) {
    let file = File::open(path).unwrap_or_else(|_| panic!("can't open {}", path.to_str().unwrap()));
    let file = BufReader::new(file);
    let parser = EventReader::new(file);
    let mut file_name = String::from("unknown");
    let mut sources: Vec<String> = vec!["".to_string()];
    let mut collect_source = false;
    for event in parser {
        match event.unwrap() {
            XmlEvent::StartElement {
                ref name,
                ref attributes,
                ..
            } => {
                collect_source = name.local_name == "source";
                if name.local_name == "class" {
                    for attribute in attributes {
                        if attribute.name.local_name == "filename" {
                            file_name = canonical_file_name(&sources, &attribute.value).unwrap();
                            coverage_annotations
                                .entry(file_name.clone())
                                .or_insert_with(HashMap::new);
                        }
                    }
                }
                if name.local_name == "line" {
                    let mut line_number = -1;
                    let mut hits_count = -1;
                    for attribute in attributes {
                        if attribute.name.local_name == "number" {
                            line_number = attribute.value.parse().unwrap();
                        } else if attribute.name.local_name == "hits" {
                            hits_count = attribute.value.parse().unwrap();
                        }
                    }
                    if line_number > 0 {
                        if hits_count == 0 {
                            coverage_annotations
                                .get_mut(&file_name)
                                .unwrap()
                                .entry(line_number)
                                .or_insert(false);
                        } else {
                            coverage_annotations
                                .get_mut(&file_name)
                                .unwrap()
                                .insert(line_number, true);
                        }
                    }
                }
            }
            XmlEvent::Characters(mut string) => {
                if collect_source {
                    if !string.ends_with('/') {
                        string.push('/');
                    }
                    sources.push(string);
                }
            }
            _ => {}
        };
    }
}

#[doc(hidden)]
fn canonical_file_name(sources: &[String], file_name: &str) -> Option<String> {
    for source in sources {
        let mut path = PathBuf::from(source);
        path.push(file_name);
        if let Ok(canonical) = fs::canonicalize(path) {
            return Some(canonical.as_path().to_str().unwrap().to_string());
        }
    }
    None
}

#[doc(hidden)]
fn report_wrong_annotations(
    flaky_policy: FlakyPolicy,
    coverage_annotations: &HashMap<String, HashMap<i32, bool>>,
    source_annotations: &HashMap<String, FileAnnotations>,
) -> i32 {
    let mut src = "src".to_string();
    if let Ok(canonical_src) = fs::canonicalize("src") {
        src = canonical_src.as_path().to_str().unwrap().to_string();
    }
    let mut tests = "tests".to_string();
    if let Ok(canonical_tests) = fs::canonicalize("tests") {
        tests = canonical_tests.as_path().to_str().unwrap().to_string();
    }
    let mut exit_status = 0;
    for (file_name, coverage_line_annotations) in coverage_annotations {
        if (file_name.starts_with(src.as_str()) || file_name.starts_with(tests.as_str()))
            && report_file_wrong_annotations(
                flaky_policy,
                file_name,
                coverage_line_annotations,
                source_annotations.get(file_name).unwrap(),
            )
        {
            exit_status = 1;
        }
    }
    for (file_name, source_file_annotations) in source_annotations {
        if (file_name.starts_with(src.as_str()) || file_name.starts_with(tests.as_str()))
            && coverage_annotations.get(file_name).is_none()
            && report_uncovered_file_annotations(file_name, source_file_annotations)
        {
            exit_status = 1;
        }
    }
    exit_status
}

#[doc(hidden)]
fn report_file_wrong_annotations(
    flaky_policy: FlakyPolicy,
    file_name: &str,
    coverage_file_annotations: &HashMap<i32, bool>,
    source_file_annotation: &FileAnnotations,
) -> bool {
    match *source_file_annotation {
        FileAnnotations::MaybeTested => false,
        FileAnnotations::NotTested => {
            eprintln!("{}: wrong FILE NOT TESTED coverage annotation", file_name);
            true
        }
        FileAnnotations::LineAnnotations(ref source_line_annotations) => {
            let mut did_report_annotation = false;
            for (mut line_number, source_line_annotation) in
                source_line_annotations.iter().enumerate()
            {
                line_number += 1;
                let coverage_line_annotation = coverage_file_annotations.get(&(line_number as i32));
                match (
                    flaky_policy,
                    source_line_annotation,
                    coverage_line_annotation,
                ) {
                    (_, &LineAnnotation::Tested(_), Some(&false))
                    | (FlakyPolicy::Tested, &LineAnnotation::FlakyTested(_), Some(&false)) => {
                        eprintln!(
                            "{}:{}: wrong TESTED coverage annotation",
                            file_name, line_number,
                        );
                        did_report_annotation = true;
                    }

                    (_, &LineAnnotation::NotTested(_), Some(&true))
                    | (FlakyPolicy::NotTested, &LineAnnotation::FlakyTested(_), Some(&true)) => {
                        eprintln!(
                            "{}:{}: wrong NOT TESTED coverage annotation",
                            file_name, line_number,
                        );
                        did_report_annotation = true;
                    }

                    (_, &LineAnnotation::Tested(true), None) => {
                        eprintln!(
                            "{}:{}: explicit TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        did_report_annotation = true;
                    }

                    (_, &LineAnnotation::NotTested(true), None) => {
                        eprintln!(
                            "{}:{}: explicit NOT TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        did_report_annotation = true;
                    }

                    (_, &LineAnnotation::MaybeTested(true), None) => {
                        eprintln!(
                            "{}:{}: explicit MAYBE TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        did_report_annotation = true;
                    }

                    _ => {}
                }
            }
            did_report_annotation
        }
    }
}

#[doc(hidden)]
fn report_uncovered_file_annotations(
    file_name: &str,
    source_file_annotations: &FileAnnotations,
) -> bool {
    match *source_file_annotations {
        FileAnnotations::MaybeTested | FileAnnotations::NotTested => false,
        FileAnnotations::LineAnnotations(_) => {
            eprintln!("{}: missing FILE NOT TESTED coverage annotation", file_name);
            true
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FlakyPolicy {
    NotTested,
    MaybeTested,
    Tested,
}

#[doc(hidden)]
fn process_args() -> FlakyPolicy {
    let mut flaky_policy = FlakyPolicy::MaybeTested;
    let mut args = std::env::args();
    args.next();
    let program = args.next().unwrap();
    for arg in args {
        match arg.as_str() {
            "--version" => {
                println!("cargo-coverage-annotations {}", VERSION);
                std::process::exit(0);
            }
            "--flaky=not-tested" => {
                flaky_policy = FlakyPolicy::NotTested;
            }
            "--flaky=maybe-tested" => {
                flaky_policy = FlakyPolicy::MaybeTested;
            }
            "--flaky=tested" => {
                flaky_policy = FlakyPolicy::Tested;
            }
            arg => {
                eprintln!(
                    "{}: unknown flag \"{}\"; valid flags are --version and --flaky=not-tested/maybe-tested/tested",
                    program,
                    arg
                );
                std::process::exit(1);
            }
        }
    }
    flaky_policy
}
