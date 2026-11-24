use can_usb_protocol::{Bitrate, Mode};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Bit rate in kbit/s: 125, 250, 500 or 1000
    #[arg(short, long, global = true, value_name = "KBIT/S", default_value = "500", value_parser = parse_bitrate)]
    pub bitrate: Bitrate,

    /// Mode: normal, listen-only (never transmits or acknowledges) or
    /// loopback (sent frames come back without reaching the bus)
    #[arg(short, long, global = true, default_value = "normal", value_parser = parse_mode)]
    pub mode: Mode,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print the interface's protocol version, firmware version and serial
    /// number
    Info,

    /// Listen to CAN frames and print them out to the console
    Listen,

    /// Send Ping-frame and expect Pong
    Ping {
        /// Period in seconds between pings
        #[arg(short, long, value_name = "SECONDS", default_value = "5")]
        period: u64,
    },

    /// Test throughput by sending ping messages as fast as possible
    Throughput,
}

fn parse_bitrate(kbit: &str) -> Result<Bitrate, String> {
    kbit.parse::<u32>()
        .ok()
        .and_then(|kbit| kbit.checked_mul(1000))
        .and_then(Bitrate::from_bits_per_second)
        .ok_or_else(|| format!("{kbit} is not 125, 250, 500 or 1000"))
}

fn parse_mode(mode: &str) -> Result<Mode, String> {
    match mode {
        "normal" => Ok(Mode::Normal),
        "listen-only" => Ok(Mode::ListenOnly),
        "loopback" => Ok(Mode::InternalLoopback),
        _ => Err(format!("{mode} is not normal, listen-only or loopback")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_bitrate_in_kbit() {
        for rate in Bitrate::ALL {
            assert_eq!(
                parse_bitrate(&(rate.bits_per_second() / 1000).to_string()),
                Ok(rate)
            );
        }
        for bad in ["", "100", "500000", "x", "-500", "4294968"] {
            assert!(parse_bitrate(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parses_every_mode() {
        assert_eq!(parse_mode("normal"), Ok(Mode::Normal));
        assert_eq!(parse_mode("listen-only"), Ok(Mode::ListenOnly));
        assert_eq!(parse_mode("loopback"), Ok(Mode::InternalLoopback));
        assert!(parse_mode("Normal").is_err());
    }

    #[test]
    fn defaults_to_500_kbit_normal() {
        let args = Args::try_parse_from(["can-usb", "listen"]).unwrap();

        assert_eq!(args.bitrate, Bitrate::Kbit500);
        assert_eq!(args.mode, Mode::Normal);

        let args =
            Args::try_parse_from(["can-usb", "listen", "--bitrate", "250", "-m", "listen-only"])
                .unwrap();

        assert_eq!(args.bitrate, Bitrate::Kbit250);
        assert_eq!(args.mode, Mode::ListenOnly);
    }
}
