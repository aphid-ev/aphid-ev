use args::{Args, Command};
use can_usb_host::*;
use can_usb_protocol::Frame;
use clap::Parser;
use std::time::Duration;
use tokio::time::Instant;

mod args;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    app(args).await.expect("Error");
}

async fn app(args: Args) -> Result<()> {
    let device_info = UsbCanDevice::find().ok_or(Error::DeviceNotFound)?;
    let usb_can = UsbCanDevice::open(device_info)?;

    match args.command {
        Command::Listen => listen(usb_can).await,
        Command::Ping { period } => ping(usb_can, Duration::from_secs(period)).await,
        Command::Throughput => throughput(usb_can).await,
    }
}

async fn listen(mut usb_can: UsbCanDevice) -> Result<()> {
    loop {
        let frame = usb_can.receive().await?;
        println!("{frame}");
    }
}

async fn ping(mut usb_can: UsbCanDevice, period: Duration) -> Result<()> {
    let ping = Frame::new(0x7ffu16, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    loop {
        _ = usb_can.send(&ping).await;

        let pong = usb_can.receive().await?;

        println!("{pong}");

        tokio::time::sleep(period).await;
    }
}

async fn throughput(mut usb_can: UsbCanDevice) -> Result<()> {
    let ping = Frame::new(0x7ffu16, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    let start = Instant::now();
    for _ in 0..1000 {
        _ = usb_can.send(&ping).await;

        let pong = usb_can.receive().await?;

        println!("{pong}");
    }
    let total = start.elapsed();

    println!(
        "Sent and received {} frames/s",
        1000.0 / total.as_secs_f64()
    );

    Ok(())
}
