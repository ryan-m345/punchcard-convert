mod csv;
mod punch;

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[derive(Clone, Copy)]
enum Format {
    Punch,
    Csv,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        let prog = args.first().map(String::as_str).unwrap_or("punchcard-convert");
        eprintln!("usage: {} <input-file> <output-file>", prog);
        eprintln!();
        eprintln!("the format on each side is inferred from the extension (.punch or .csv)");
        return ExitCode::FAILURE;
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let (input_format, output_format) = match (extension_of(input_path), extension_of(output_path)) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("error: expected both files to end in .punch or .csv");
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

    let entries = match input_format {
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

    let rendered = match output_format {
        Format::Punch => punch::write(&entries),
        Format::Csv => csv::write(&entries),
    };

    if let Err(e) = fs::write(output_path, rendered) {
        eprintln!("error: couldn't write \"{}\": {}", output_path, e);
        return ExitCode::FAILURE;
    }

    eprintln!("wrote {} entries to {}", entries.len(), output_path);
    ExitCode::SUCCESS
}

fn extension_of(path: &str) -> Option<Format> {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("punch") => Some(Format::Punch),
        Some("csv") => Some(Format::Csv),
        _ => None,
    }
}
