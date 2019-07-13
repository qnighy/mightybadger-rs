use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;

use failure::Backtrace;

use crate::payload::BacktraceEntry;

#[derive(Debug, Clone)]
pub struct BacktraceLine {
    pub line: Option<u32>,
    pub file: Option<String>,
    pub method: String,
}

pub fn parse(bt: &Backtrace) -> Vec<BacktraceLine> {
    let bt = bt.to_string();

    let mut last_file: Option<(String, u32)> = None;
    let mut last_method: Option<String> = None;
    let mut bt_lines = Vec::new();
    macro_rules! flush {
        () => {
            if let Some(method) = last_method.take() {
                let (file, line) = if let Some((file, line)) = last_file.take() {
                    (Some(file), Some(line))
                } else {
                    (None, None)
                };
                bt_lines.push(BacktraceLine { line, file, method });
            } else {
                last_file.take();
            }
        };
    };

    for line in bt.lines() {
        let line = line.trim();
        if line == "stack backtrace:" {
            continue;
        }

        // Skip "<frameno>:"
        let line = if line.chars().nth(0).unwrap_or(' ').is_numeric() {
            let pos = line.find(':').map(|x| x + 1).unwrap_or(line.len());
            &line[pos..]
        } else {
            line
        };
        let line = line.trim_start();

        // Skip "0x<ptr>"
        let line = if line.starts_with("0x") {
            let line = &line["0x".len()..];
            let pos = line.find(|c: char| !c.is_digit(16)).unwrap_or(line.len());
            &line[pos..]
        } else {
            line
        };
        let line = line.trim_start();

        // Skip "-"
        let line = if line.starts_with("-") {
            &line["-".len()..]
        } else {
            line
        };
        let line = line.trim_start();

        if line == "" {
            continue;
        }

        // at <file>:<line>
        if line.starts_with("at ") {
            let line = &line["at ".len()..];
            let line = line.trim_start();
            if let Some(pos) = line.rfind(':') {
                last_file = Some((
                    line[..pos].to_string(),
                    line[pos + ":".len()..].parse().unwrap_or(1),
                ));
            } else {
                last_file = Some((line.to_string(), 1));
            }
            continue;
        }

        // Flash last line here
        flush!();

        // Remaining Possibilities:
        // - <unresolved>
        // - <no info>
        // - <unknown>
        // - method DefPath
        last_method = Some(line.to_string());
    }
    flush!();
    bt_lines
}

pub fn trim_backtrace(bt_lines: &mut Vec<BacktraceLine>) {
    let trim_paths = [
        "honeybadger::notify::",
        "backtrace::backtrace::capture::Backtrace::new::",
        "backtrace::backtrace::capture::Backtrace::new_unresolved::",
        "failure::backtrace::Backtrace::new::",
        "<failure::backtrace::Backtrace as core::default::Default>::default::",
        "failure::failure::error_message::err_msg::",
        "<failure::context::Context<D>>::new::",
        "std::panicking::begin_panic::",
        "core::panicking::panic::",
        "core::panicking::panic_bounds_check::",
        "<core::option::Option<T>>::unwrap::",
        "<core::option::Option<T>>::expect::",
        "<core::result::Result<T, E>>::unwrap::",
        "<core::result::Result<T, E>>::expect::",
        "<core::result::Result<T, E>>::unwrap_err::",
        "<core::result::Result<T, E>>::expect_err::",
    ];
    let pos = bt_lines
        .iter()
        .rposition(|bt_line| {
            trim_paths
                .iter()
                .any(|trim_path| bt_line.method.starts_with(trim_path))
        })
        .map(|x| x + 1)
        .unwrap_or(0);

    bt_lines.drain(..pos);
}

pub fn decorate(bt_lines: Vec<BacktraceLine>) -> Vec<BacktraceEntry> {
    bt_lines
        .into_iter()
        .map(|bt_line| {
            let source = if let (Some(line), &Some(ref file)) = (bt_line.line, &bt_line.file) {
                let line = line.saturating_sub(1);
                let skip = line.saturating_sub(2);
                let upto = line.saturating_add(3);
                if let Ok(file) = File::open(&file) {
                    let mut source = BTreeMap::new();
                    let mut file = BufReader::new(file);
                    let mut line = String::new();
                    for lineno in 0..upto {
                        line.clear();
                        if let Ok(num_read) = file.read_line(&mut line) {
                            if num_read == 0 {
                                break;
                            }
                        } else {
                            break;
                        }
                        if lineno >= skip {
                            let lineno = lineno.saturating_add(1);
                            let line = mem::replace(&mut line, String::new());
                            source.insert(lineno, line);
                        }
                    }
                    Some(source)
                } else {
                    None
                }
            } else {
                None
            };
            BacktraceEntry {
                number: bt_line.line.map(|line| line.to_string()),
                file: bt_line.file,
                method: bt_line.method,
                source: source,
            }
        })
        .collect::<Vec<_>>()
}

pub fn parse_and_decorate(bt: &Backtrace) -> Vec<BacktraceEntry> {
    let mut bt_lines = parse(bt);
    trim_backtrace(&mut bt_lines);
    decorate(bt_lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_backtrace() {
        fn f() {
            let (bt, line) = (Backtrace::new(), line!());
            let bt_lines = parse(&bt);
            // eprintln!("bt_lines = {:#?}", bt_lines);
            assert!(bt_lines.iter().any(|bt_line| {
                let method_ok = bt_line
                    .method
                    .starts_with("honeybadger::btparse::tests::test_backtrace::f::");
                let file_ok = bt_line
                    .file
                    .as_ref()
                    .map(|file| file.ends_with("/btparse.rs"))
                    .unwrap_or(false);
                let line_ok = bt_line.line == Some(line);
                method_ok && file_ok && line_ok
            }));
        }
        env::set_var("RUST_BACKTRACE", "1");
        f();
    }
}
