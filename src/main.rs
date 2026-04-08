use resize_to_bleed_or_trim_pdf::process_pdf;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

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

    if positional.is_empty() || positional.len() > 2 {
        eprintln!(
            "Usage: {} [-b <thousandths>] [-t] <input.pdf> [output.pdf]",
            args[0]
        );
        process::exit(1);
    }

    if trim && bleed_thousandths.is_some() {
        eprintln!("Error: -b and -t are mutually exclusive");
        process::exit(1);
    }

    let bleed_points: f64 = match bleed_thousandths {
        Some(num) => num as f64 / 1000.0 * 72.0,
        None => 0.0,
    };

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
