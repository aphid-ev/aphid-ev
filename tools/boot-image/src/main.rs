use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

/// Seal a linked application ELF for the bootloader.
///
/// Fills in the image header's length and CRC, writes the sealed header back
/// into the ELF so it can be flashed with a debug probe, and writes the raw
/// image for flashing over CAN.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// The application ELF, patched in place.
    elf: PathBuf,
    /// Where to write the raw image. Defaults to the ELF's path with `.bin`.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let output = args
        .output
        .unwrap_or_else(|| args.elf.with_extension("bin"));

    match run(&args.elf, &output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("boot-image: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(elf_path: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let mut elf = std::fs::read(elf_path).map_err(|e| format!("{}: {e}", elf_path.display()))?;
    let sealed = boot_image::seal(&mut elf).map_err(|e| format!("{}: {e}", elf_path.display()))?;

    std::fs::write(elf_path, &elf).map_err(|e| format!("{}: {e}", elf_path.display()))?;
    std::fs::write(output, &sealed.image).map_err(|e| format!("{}: {e}", output.display()))?;

    let version = sealed.version;
    println!(
        "{}: version {}.{}.{}, {} bytes at {:#010x}, CRC {:#010x}",
        output.display(),
        version.major,
        version.minor,
        version.patch,
        sealed.image.len(),
        sealed.start,
        sealed.crc,
    );

    Ok(())
}
