use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use boot_protocol::{ImageError, NodeId, SemVer};
use clap::{Parser, Subcommand};
use manager::update::{self, Image, Step, Timeouts};
use manager::usb::UsbBus;

/// Manage the ECUs over CAN through the CAN/USB interface.
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Flash a sealed application image into a node through its bootloader.
    ///
    /// Reset or power-cycle the node after starting this: the update waits
    /// for it. A node left in its bootloader by an earlier, interrupted
    /// update is found without a reset.
    Flash {
        /// The node's ID, 0 to 14.
        #[arg(short, long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..15))]
        node: u8,
        /// How long to wait for the node, in seconds.
        #[arg(long, default_value_t = 30)]
        wait: u64,
        /// The sealed raw image, such as can-echo.bin.
        image: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let result = match Args::parse().command {
        Commands::Flash { node, wait, image } => flash(node, wait, &image).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("manager: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn flash(node: u8, wait: u64, path: &PathBuf) -> Result<(), String> {
    let node = NodeId::new(node).ok_or("node IDs run from 0 to 14")?;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let image = Image::new(bytes).map_err(|e| format!("{}: {}", path.display(), image_error(e)))?;
    let timeouts = Timeouts {
        catch: Duration::from_secs(wait),
        ..Timeouts::default()
    };

    println!(
        "{}: version {}, {} bytes",
        path.display(),
        version(image.version()),
        image.len()
    );

    let mut bus = UsbBus::open().map_err(|e| e.to_string())?;

    update::update(&mut bus, node, &image, &timeouts, report)
        .await
        .map_err(|e| e.to_string())
}

fn report(step: Step) {
    match step {
        Step::Catching => println!("Waiting for the node. Reset it now."),
        Step::Caught { wakeup: None } => println!("Found the node in its bootloader."),
        Step::Caught {
            wakeup: Some(Ok(old)),
        } => println!("Caught the node, running version {}.", version(old)),
        Step::Caught {
            wakeup: Some(Err(error)),
        } => println!(
            "Caught the node, without a valid image: {}.",
            image_error(error)
        ),
        Step::Erasing => println!("Erasing."),
        Step::Programming { done, total } => println!("Programmed {done} of {total} bytes."),
        Step::Starting => println!("Starting the application."),
        Step::Done => println!("Done."),
    }
}

fn version(version: SemVer) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

fn image_error(error: ImageError) -> &'static str {
    match error {
        ImageError::Missing => "no image header",
        ImageError::Length => "the image is not sealed or its length is wrong",
        ImageError::Checksum => "the image does not match its CRC",
    }
}
