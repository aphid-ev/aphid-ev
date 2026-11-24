use args::{Args, Command};
use boot_protocol::BUS_TEST_ID;
use can_usb_host::*;
use can_usb_protocol::{Bitrate, Config, Frame};
use clap::Parser;
use std::time::Duration;
use throughput::Tally;
use tokio::time::{Instant, timeout};

mod args;
mod throughput;

/// How long `throughput` waits for the interface to take a frame, or for the
/// next event once everything is sent, before it gives up.
const IDLE: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() {
    let args = Args::parse();

    app(args).await.expect("Error");
}

async fn app(args: Args) -> Result<()> {
    let device_info = UsbCanDevice::find().ok_or(Error::DeviceNotFound)?;

    // Every command sets the config, so the bus runs as asked whatever an
    // earlier program left configured.
    let config = Config {
        bitrate: args.bitrate,
        mode: args.mode,
    };
    let usb_can = UsbCanDevice::open_with_config(device_info, config)?;

    match args.command {
        Command::Info => info(&usb_can).await,
        Command::Listen => listen(usb_can).await,
        Command::Ping { period } => ping(usb_can, Duration::from_secs(period)).await,
        Command::Throughput { count } => throughput(usb_can, args.bitrate, count).await,
    }
}

async fn info(usb_can: &UsbCanDevice) -> Result<()> {
    let info = usb_can.device_info().await?;
    let serial: String = info
        .serial
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect();

    println!("Protocol version: {}", info.protocol_version);
    println!("Firmware version: {}", info.firmware_version);
    println!("Serial number:    {serial}");

    Ok(())
}

async fn listen(mut usb_can: UsbCanDevice) -> Result<()> {
    loop {
        let frame = usb_can.receive().await?;
        println!("{frame}");
    }
}

async fn ping(mut usb_can: UsbCanDevice, period: Duration) -> Result<()> {
    let ping = Frame::new(BUS_TEST_ID, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    loop {
        usb_can.send(&ping).await?;

        let pong = usb_can.receive().await?;

        println!("{pong}");

        tokio::time::sleep(period).await;
    }
}

/// Sends `count` frames as fast as the interface takes them, collecting
/// events in between, then waits for the rest.
async fn throughput(mut usb_can: UsbCanDevice, bitrate: Bitrate, count: u32) -> Result<()> {
    let mut tally = Tally::new(BUS_TEST_ID);
    let start = Instant::now();
    let mut sent = 0;

    for n in 0..count {
        let frame = throughput::test_frame(BUS_TEST_ID, n);

        match timeout(IDLE, usb_can.send(&frame)).await {
            Ok(result) => result?,
            Err(_) => {
                println!(
                    "The interface stopped taking frames after {sent}. \
                     Does another node acknowledge them?"
                );
                break;
            }
        }
        sent += 1;

        while let Some(event) = usb_can.try_receive() {
            tally.add(event?);
        }
    }
    let sending = start.elapsed();

    if sent == count {
        timeout(IDLE, usb_can.flush()).await.ok().transpose()?;
    }

    while tally.frames < u64::from(count) {
        match timeout(IDLE, usb_can.receive()).await {
            Ok(event) => tally.add(event?),
            Err(_) => break,
        }
    }

    println!(
        "Sent       {sent} frames in {:.2} s, {:.0} frames/s",
        sending.as_secs_f64(),
        f64::from(sent) / sending.as_secs_f64()
    );
    println!(
        "Received   {} frames, {} lost in overruns, {} bus errors, {} out of sequence",
        tally.frames, tally.lost, tally.bus_errors, tally.out_of_sequence
    );

    match (
        tally.frames_per_second(),
        tally.bus_load_percent(bitrate.bits_per_second()),
    ) {
        (Some(rate), Some(load)) => println!(
            "Rate       {rate:.0} frames/s on the interface's clock, \
             bus load {load:.1} % without stuff bits"
        ),
        _ => println!("Rate       too few frames received to measure"),
    }

    Ok(())
}
