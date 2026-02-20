# wooden-box

ESP32 firmware written in Rust. Connects to Home Assistant via MQTT.

## Setup

### Prerequisites

- [espup](https://github.com/esp-rs/espup) installed and run once (`espup install`)
- [espflash](https://github.com/esp-rs/espflash) for flashing (`cargo install espflash`)

### Before building

The Xtensa toolchain must be on your PATH. Run this in every new terminal session before building or flashing:

```sh
. ~/export-esp.sh
```

### Configuration

Copy the example config and fill in your values:

```sh
cp src/config.rs.example src/config.rs
```

Edit `src/config.rs` with your WiFi credentials and MQTT broker IP.

## Build & Flash

```sh
cargo build                        # compile only
cargo run                          # flash and open serial monitor
```
