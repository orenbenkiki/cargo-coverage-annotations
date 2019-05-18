// Copyright (C) 2017,2018,2019 Oren Ben-Kiki <oren@ben-kiki.org>
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

//! Ensure annotations in code match actual coverage.

extern crate regex;
#[macro_use]
extern crate version;
extern crate xml;

use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::vec::Vec;
use xml::reader::{EventReader, XmlEvent};

enum LineMark {
    None,
    LineTested,
    LineMaybeTested,
    LineNotTested,
    BeginMaybeTested,
    BeginNotTested,
    EndMaybeTested,
    EndNotTested,
    FileMaybeTested,
    FileNotTested,
}

#[derive(Clone)]
enum LineAnnotation {
    Tested(bool),
    MaybeTested(bool),
    NotTested(bool),
}

fn is_explicit(line_annotation: &LineAnnotation) -> bool {
    match *line_annotation {
        LineAnnotation::Tested(true) => true,
        LineAnnotation::MaybeTested(true) => true,
        LineAnnotation::NotTested(true) => true,
        _ => false,
    }
}

enum FileAnnotations {
    LineAnnotations(Vec<LineAnnotation>),
    MaybeTested,
    NotTested,
}

fn main() {
    process_args();

    let mut coverage_annotations = HashMap::new();
    let mut source_annotations = HashMap::new();
    collect_dir_annotations(
        Path::new("."),
        &mut source_annotations,
        &mut coverage_annotations,
    )
    .unwrap();
    let exit_status = report_wrong_annotations(&coverage_annotations, &source_annotations);
    std::process::exit(exit_status);
}

fn collect_dir_annotations(
    dir: &Path,
    source_annotations: &mut HashMap<String, FileAnnotations>,
    coverage_annotations: &mut HashMap<String, HashMap<i32, bool>>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_annotations(&path, source_annotations, coverage_annotations)?;
        } else if let Ok(canonical) = fs::canonicalize(path) {
            let file_name = canonical.as_path().to_str().unwrap();
            if file_name.ends_with("/cobertura.xml") {
                collect_coverage_annotations(&canonical.as_path(), coverage_annotations);
            } else if file_name.ends_with(".rs") {
                let annotations = collect_file_annotations(&canonical.as_path())?;
                source_annotations.insert(file_name.to_string(), annotations);
            }
        }
    }
    Ok(())
}

fn collect_file_annotations(path: &Path) -> std::io::Result<FileAnnotations> {
    let file = File::open(path).unwrap_or_else(|_| panic!("can't open {}", path.to_str().unwrap()));
    let file = BufReader::new(file);
    let mut region_annotation = LineAnnotation::Tested(false);
    let mut is_file_not_tested = false;
    let mut is_file_maybe_tested = false;
    let mut line_annotations = Vec::new();
    let untrusted_regex =
        Regex::new(r"^(?:\s*\}(?:\s*\))*(?:\s*;)?|\s*(?:\}\s*)?else(?:\s*\{)?)?\s*(?://.*)?$")
            .unwrap();
    for (mut line_number, line) in file.lines().enumerate() {
        line_number += 1;
        let line_text = line.unwrap();
        let line_mark = extract_line_mark(line_text.as_ref());
        let (line_annotation, next_region_annotation) = match (line_mark, region_annotation) {
            (LineMark::None, region_annotation) => {
                if untrusted_regex.is_match(line_text.as_ref()) {
                    (LineAnnotation::MaybeTested(false), region_annotation)
                } else {
                    (region_annotation.clone(), region_annotation)
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
                (region_annotation.clone(), region_annotation)
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
                (region_annotation.clone(), region_annotation)
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
                (region_annotation.clone(), region_annotation)
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
                (region_annotation.clone(), region_annotation)
            }

            (LineMark::FileMaybeTested, region_annotation) => {
                if is_file_not_tested || is_file_maybe_tested {
                    eprintln!(
                        "{}:{}: repeated FILE MAYBE TESTED coverage annotation",
                        path.to_str().unwrap(),
                        line_number
                    );
                }
                is_file_maybe_tested = true;
                (region_annotation.clone(), region_annotation)
            }

            (LineMark::FileNotTested, region_annotation) => {
                if is_file_not_tested || is_file_maybe_tested {
                    eprintln!(
                        "{}:{}: repeated FILE NOT TESTED coverage annotation",
                        path.to_str().unwrap(),
                        line_number
                    );
                }
                is_file_not_tested = true;
                (region_annotation.clone(), region_annotation)
            }
        };
        line_annotations.push(line_annotation);
        region_annotation = next_region_annotation;
    }
    if is_file_maybe_tested {
        verify_untested_file_annotations(path, &line_annotations);
        Ok(FileAnnotations::MaybeTested)
    } else if is_file_not_tested {
        verify_untested_file_annotations(path, &line_annotations);
        Ok(FileAnnotations::NotTested)
    } else {
        Ok(FileAnnotations::LineAnnotations(line_annotations))
    }
}

fn verify_untested_file_annotations(path: &Path, line_annotations: &[LineAnnotation]) {
    for (mut line_number, line_annotation) in line_annotations.iter().enumerate() {
        line_number += 1;
        if is_explicit(line_annotation) {
            eprintln!(
                "{}:{}: line coverage annotation in a FILE which is NOT TESTED",
                path.to_str().unwrap(),
                line_number
            );
        }
    }
}

fn extract_line_mark(line: &str) -> LineMark {
    if line.ends_with("// TESTED") {
        LineMark::LineTested
    } else if line.ends_with("// MAYBE TESTED") {
        LineMark::LineMaybeTested
    } else if line.ends_with("// NOT TESTED") {
        LineMark::LineNotTested
    } else if line.ends_with("// BEGIN MAYBE TESTED") {
        LineMark::BeginMaybeTested
    } else if line.ends_with("// BEGIN NOT TESTED") {
        LineMark::BeginNotTested
    } else if line.ends_with("// END MAYBE TESTED") {
        LineMark::EndMaybeTested
    } else if line.ends_with("// END NOT TESTED") {
        LineMark::EndNotTested
    } else if line.ends_with("// FILE MAYBE TESTED") {
        LineMark::FileMaybeTested
    } else if line.ends_with("// FILE NOT TESTED") {
        LineMark::FileNotTested
    } else {
        LineMark::None
    }
}

fn collect_coverage_annotations(
    path: &Path,
    coverage_annotations: &mut HashMap<String, HashMap<i32, bool>>,
) {
    let file = File::open(path).unwrap_or_else(|_| panic!("can't open {}", path.to_str().unwrap()));
    let file = BufReader::new(file);
    let parser = EventReader::new(file);
    let mut file_name = String::from("unknown");
    for event in parser {
        if let XmlEvent::StartElement {
            ref name,
            ref attributes,
            ..
        } = event.unwrap()
        {
            if name.local_name == "class" {
                for attribute in attributes {
                    if attribute.name.local_name == "filename" {
                        let canonical = fs::canonicalize(attribute.value.clone()).unwrap();
                        file_name = canonical.as_path().to_str().unwrap().to_string();
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
        };
    }
}

fn report_wrong_annotations(
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

fn report_file_wrong_annotations(
    file_name: &str,
    coverage_file_annotations: &HashMap<i32, bool>,
    source_file_annotation: &FileAnnotations,
) -> bool {
    match *source_file_annotation {
        FileAnnotations::MaybeTested => true,
        FileAnnotations::NotTested => {
            eprintln!("{}: wrong FILE NOT TESTED coverage annotation", file_name);
            false
        }
        FileAnnotations::LineAnnotations(ref source_line_annotations) => {
            let mut has_wrong_annotation = false;
            for (mut line_number, source_line_annotation) in
                source_line_annotations.iter().enumerate()
            {
                line_number += 1;
                let coverage_line_annotation = coverage_file_annotations.get(&(line_number as i32));
                match (source_line_annotation, coverage_line_annotation) {
                    (&LineAnnotation::Tested(_), Some(&false)) => {
                        eprintln!(
                            "{}:{}: wrong TESTED coverage annotation",
                            file_name, line_number,
                        );
                        has_wrong_annotation = true;
                    }

                    (&LineAnnotation::NotTested(_), Some(&true)) => {
                        eprintln!(
                            "{}:{}: wrong NOT TESTED coverage annotation",
                            file_name, line_number,
                        );
                        has_wrong_annotation = true;
                    }

                    (&LineAnnotation::Tested(true), None) => {
                        eprintln!(
                            "{}:{}: explicit TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        has_wrong_annotation = true;
                    }

                    (&LineAnnotation::NotTested(true), None) => {
                        eprintln!(
                            "{}:{}: \
                             explicit NOT TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        has_wrong_annotation = true;
                    }

                    (&LineAnnotation::MaybeTested(true), None) => {
                        eprintln!(
                            "{}:{}: \
                             explicit MAYBE TESTED coverage annotation for a non-executable line",
                            file_name, line_number,
                        );
                        has_wrong_annotation = true;
                    }

                    _ => {}
                }
            }
            has_wrong_annotation
        }
    }
}

fn report_uncovered_file_annotations(
    file_name: &str,
    source_file_annotations: &FileAnnotations,
) -> bool {
    match *source_file_annotations {
        FileAnnotations::MaybeTested => false,
        FileAnnotations::NotTested => false,
        FileAnnotations::LineAnnotations(_) => {
            eprintln!("{}: missing FILE NOT TESTED coverage annotation", file_name);
            true
        }
    }
}

fn process_args() {
    let count = std::env::args().count();
    let mut args = std::env::args();
    let mut are_args_valid = true;
    let mut should_print_version = false;

    args.nth(0);
    match count {
        1 => {}
        2 => match args.nth(0).unwrap().as_ref() {
            "--version" => {
                should_print_version = true;
            }
            "coverage-annotations" => {}
            _ => {
                are_args_valid = false;
            }
        },
        3 => {
            if args.nth(0).unwrap() == "coverage-annotations" && args.nth(0).unwrap() == "--version"
            {
                should_print_version = true;
            } else {
                are_args_valid = false;
            }
        }
        _ => {
            are_args_valid = false;
        }
    }

    if !are_args_valid {
        println!("cargo-coverage-annotations takes no arguments (except --version).");
        std::process::exit(1);
    }

    if should_print_version {
        println!("cargo-coverage-annotations {}", version!());
        std::process::exit(0);
    }
}
