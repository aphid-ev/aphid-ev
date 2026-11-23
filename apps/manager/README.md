# Manager

The desktop app's UI-free core library and its command line. The egui
front end comes later. The core keeps the CAN bus behind the `Bus` trait,
with the CAN/USB interface as one implementation and a fake bus with a
simulated bootloader node for the tests.

```sh
cargo run -p manager -- --help
cargo run -p manager -- flash --node 0 <image.bin>
just host ci
```

The firmware update follows the protocol on
[the bootloader wiki page](https://bladlus.se/wiki/bootloader/).
