use failure::Backtrace;

#[derive(Debug, Clone)]
pub struct BacktraceLine {
    pub line: u32,
    pub file: String,
    pub method: String,
}

pub fn parse(bt: &Backtrace) -> Vec<BacktraceLine> {
    let bt = bt.to_string();

    let mut last_file: Option<(String, u32)> = None;
    let mut last_method: Option<String> = None;
    let mut bt_lines = Vec::new();
    macro_rules! flush {
        () => {
            if last_method.is_some() || last_file.is_some() {
                let method = last_method
                    .take()
                    .unwrap_or_else(|| "<unknown>".to_string());
                let file = last_file
                    .take()
                    .unwrap_or_else(|| ("<unknown>".to_string(), 1));
                bt_lines.push(BacktraceLine {
                    line: file.1,
                    file: file.0,
                    method: method,
                });
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
        let line = line.trim_left();

        // Skip "0x<ptr>"
        let line = if line.starts_with("0x") {
            let line = &line["0x".len()..];
            let pos = line.find(|c: char| !c.is_digit(16)).unwrap_or(line.len());
            &line[pos..]
        } else {
            line
        };
        let line = line.trim_left();

        // Skip "-"
        let line = if line.starts_with("-") {
            &line["-".len()..]
        } else {
            line
        };
        let line = line.trim_left();

        if line == "" {
            continue;
        }

        // at <file>:<line>
        if line.starts_with("at ") {
            let line = &line["at ".len()..];
            let line = line.trim_left();
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

pub fn trim_failure_backtrace(bt_lines: &mut Vec<BacktraceLine>) {
    let trim_paths = [
        "backtrace::backtrace::capture::Backtrace::new::",
        "backtrace::backtrace::capture::Backtrace::new_unresolved::",
        "failure::backtrace::Backtrace::new::",
        "<failure::backtrace::Backtrace as core::default::Default>::default::",
        "failure::failure::error_message::err_msg::",
        "<failure::context::Context<D>>::new::",
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

pub fn trim_panic_backtrace(bt_lines: &mut Vec<BacktraceLine>) {
    let trim_paths = [
        "std::panicking::begin_panic::",
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
                bt_line
                    .method
                    .starts_with("honeybadger::btparse::tests::test_backtrace::f::")
                    && bt_line.file.ends_with("/btparse.rs") && bt_line.line == line
            }));
        }
        env::set_var("RUST_BACKTRACE", "1");
        f();
    }
}
