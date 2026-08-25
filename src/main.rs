mod csv;
mod punch;
mod summary;

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

#[derive(Clone, Copy)]
enum Format {
    Punch,
    Csv,
}

impl Format {
    fn opposite(self) -> Format {
        match self {
            Format::Punch => Format::Csv,
            Format::Csv => Format::Punch,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() == 3 && args[1] == "--summary" {
        return run_summary(&args[2]);
    }

    if args.len() != 3 {
        print_usage(&args);
        return ExitCode::FAILURE;
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let (input_format, output_format) = match resolve_formats(input_path, output_path) {
        Ok(pair) => pair,
        Err(msg) => {
            eprintln!("error: {}", msg);
            return ExitCode::FAILURE;
        }
    };

    let contents = match read_source(input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: couldn't read {}: {}", label(input_path), e);
            return ExitCode::FAILURE;
        }
    };

    let entries = match input_format {
        Format::Punch => punch::parse(&contents),
        Format::Csv => csv::parse(&contents),
    };
    let entries = match entries {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error in {}:", label(input_path));
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    let rendered = match output_format {
        Format::Punch => punch::write(&entries),
        Format::Csv => csv::write(&entries),
    };

    if let Err(e) = write_dest(output_path, &rendered) {
        eprintln!("error: couldn't write {}: {}", label(output_path), e);
        return ExitCode::FAILURE;
    }

    eprintln!("wrote {} entries to {}", entries.len(), label(output_path));
    ExitCode::SUCCESS
}

fn print_usage(args: &[String]) {
    let prog = args.first().map(String::as_str).unwrap_or("punchcard-convert");
    eprintln!("usage: {} <input-file> <output-file>", prog);
    eprintln!("       {} --summary <input-file>", prog);
    eprintln!();
    eprintln!("the format on each side is inferred from the extension (.punch or .csv)");
    eprintln!("use \"-\" in place of a file to read from stdin or write to stdout;");
    eprintln!("its format is inferred as the opposite of the other side's");
}

/// Works out the format on each side. A literal "-" stands for stdin (as
/// input) or stdout (as output) and has no extension of its own, so its
/// format is taken to be the opposite of whichever format the real file on
/// the other side resolves to.
fn resolve_formats(input_path: &str, output_path: &str) -> Result<(Format, Format), String> {
    match (input_path == "-", output_path == "-") {
        (true, true) => Err("can't use \"-\" for both input and output; the format can't be inferred".to_string()),
        (true, false) => {
            let out_format = extension_of(output_path)
                .ok_or_else(|| format!("expected \"{}\" to end in .punch or .csv", output_path))?;
            Ok((out_format.opposite(), out_format))
        }
        (false, true) => {
            let in_format = extension_of(input_path)
                .ok_or_else(|| format!("expected \"{}\" to end in .punch or .csv", input_path))?;
            Ok((in_format, in_format.opposite()))
        }
        (false, false) => match (extension_of(input_path), extension_of(output_path)) {
            (Some(a), Some(b)) => Ok((a, b)),
            _ => Err("expected both files to end in .punch or .csv".to_string()),
        },
    }
}

fn read_source(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

fn write_dest(path: &str, contents: &str) -> io::Result<()> {
    if path == "-" {
        io::stdout().write_all(contents.as_bytes())
    } else {
        fs::write(path, contents)
    }
}

fn label(path: &str) -> String {
    if path == "-" {
        "<stdio>".to_string()
    } else {
        format!("\"{}\"", path)
    }
}

fn run_summary(input_path: &str) -> ExitCode {
    let format = match extension_of(input_path) {
        Some(f) => f,
        None => {
            eprintln!("error: expected \"{}\" to end in .punch or .csv", input_path);
            return ExitCode::FAILURE;
        }
    };

    let contents = match fs::read_to_string(input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: couldn't read \"{}\": {}", input_path, e);
            return ExitCode::FAILURE;
        }
    };

    let entries = match format {
        Format::Punch => punch::parse(&contents),
        Format::Csv => csv::parse(&contents),
    };
    let entries = match entries {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error in {}:", input_path);
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    let report = match summary::summarize(&entries) {
        Ok(report) => report,
        Err(msg) => {
            eprintln!("error in {}: {}", input_path, msg);
            return ExitCode::FAILURE;
        }
    };

    print!("{}", summary::render(&report));
    ExitCode::SUCCESS
}

fn extension_of(path: &str) -> Option<Format> {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("punch") => Some(Format::Punch),
        Some("csv") => Some(Format::Csv),
        _ => None,
    }
}
