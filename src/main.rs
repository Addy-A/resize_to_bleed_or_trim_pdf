use resize_to_bleed_or_trim_pdf::process_pdf;
use std::path::{Path, PathBuf};
use std::process;

fn parse_batch_args(args: &[String]) -> Option<(Vec<PathBuf>, Option<PathBuf>)> {
    if !args.get(1).map_or(false, |arg| arg.starts_with('[')) {
        return None;
    }

    let mut raw_tokens = Vec::new();
    let mut closing_idx: Option<usize> = None;

    for (i, arg) in args[1..].iter().enumerate() {
        raw_tokens.push(arg.clone());
        if arg.ends_with(']') {
            closing_idx = Some(i + 1);
            break;
        }
    }

    let closing_idx = closing_idx?;

    let joined = raw_tokens.join(" ");
    let inner = joined.trim_start_matches('[').trim_end_matches(']');

    let inputs: Vec<PathBuf> = inner
        .split(',')
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    if inputs.is_empty() {
        return None;
    }

    let output_dir = args.get(closing_idx + 1).map(PathBuf::from);

    Some((inputs, output_dir))
}
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&String::from("--version")) {
        let version = env!("CARGO_PKG_VERSION").to_string();
        println!("{}", version);
        process::exit(0);
    }
    let mut bleed_thousandths: Option<u32> = None;
    let mut trim = false;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-b" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: -b requires a value");
                    process::exit(1);
                }
                bleed_thousandths = Some(args[i].parse::<u32>().unwrap_or_else(|_| {
                    eprintln!("Error: -b value must be a positive integer");
                    process::exit(1);
                }));
            }
            "-t" => {
                trim = true;
            }
            _ => {
                positional.push(args[i].clone());
            }
        }
        i += 1;
    }

    if trim && bleed_thousandths.is_some() {
        eprintln!("Error: -b and -t are mutually exclusive");
        process::exit(1);
    }

    let bleed_points: f64 = match bleed_thousandths {
        Some(num) => num as f64 / 1000.0 * 72.0,
        None => 0.0,
    };

    let mut batch_candidate = vec![String::new()];
    batch_candidate.extend_from_slice(&positional);
    if let Some((inputs, output_dir)) = parse_batch_args(&batch_candidate) {
        let mut had_error = false;

        for input_path in inputs {
            if !input_path.exists() {
                eprintln!("Error: {:?} does not exist.", input_path.display());
                had_error = true;
                continue;
            }

            let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
            let output_path = match &output_dir {
                Some(dir) => dir.join(format!("{}-resized.pdf", stem)),
                None => {
                    let parent = input_path.parent().unwrap_or(Path::new("."));
                    parent.join(format!("{}-resized.pdf", stem))
                }
            };

            if let Some(parent) = output_path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("Error: could not create output directory {}", e);
                        had_error = true;
                        continue;
                    }
                }
            }

            match process_pdf(&input_path, &output_path, bleed_points) {
                Ok(()) => println!("Saved resized PDF to {}", output_path.display()),
                Err(e) => {
                    eprintln!("Error processing PDF: {}", e);
                    process::exit(1);
                }
            }
        }
        if had_error {
            process::exit(1);
        }
    } else {
        if positional.is_empty() || positional.len() > 2 {
            eprintln!("Usage ...");
            process::exit(1);
        }

        let input_path = Path::new(&positional[0]);

        if !input_path.exists() {
            eprintln!("Error: input file not found: {}", input_path.display());
            process::exit(1);
        }

        let output_path: PathBuf = if positional.len() == 2 {
            let given = PathBuf::from(&positional[1]);
            if given.is_dir() {
                let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
                given.join(format!("{}-resized.pdf", stem))
            } else {
                given
            }
        } else {
            let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
            let input_dir = input_path.parent().unwrap_or(Path::new("."));
            input_dir.join(format!("{}-resized.pdf", stem))
        };

        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("Error: could not create output directory: {}", e);
                    process::exit(1);
                }
            }
        }

        match process_pdf(input_path, &output_path, bleed_points) {
            Ok(()) => println!("Saved resized PDF to {}", output_path.display()),
            Err(e) => {
                eprintln!("Error processing PDF: {}", e);
                process::exit(1);
            }
        }
    }
}
