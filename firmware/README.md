# E-Chess Firmware

This is the firmware for the E-Chess project, written in Rust using the ESP-IDF framework.

**This project works best with the ESP32-S3 (with 16mb flash) dev board.**
**The ESP32 is currently still in the firmware, but not recommended.**

For OTA to work, see the requirements in the [OTA Support](#ota-support) section.

## Prerequisites

- Rust (latest stable)
- ESP-IDF (see docs `espup install`)

## Quick Start

```bash
# Build, flash and watch logs (effectively cargo run)
./build.sh run esp32s3 --release

# Just watch logs via usb (if it is already running on the esp)
./build.sh watch

# Build and deploy via OTA (and optionally watch - only if connected via usb)
./build.sh ota 192.168.4.1 esp32s3 --release --watch
```

For more options and commands, run:
```bash
./build.sh --help
```

## OTA Support

For OTA updates, you need a flash size of at least 8MB *(16MB recommended!)* since the firmware is already > 2MB.
The flash must be partitioned with two slots.

**16MB is recommended!**  
8MB should also work for now, but the firmware may get bigger some day.  
Due to using rust std, this firmware is definitely not the smallest.

Either:
1. Copy the appropriate file (compatible to running with `cargo run`):
```bash
# For 8MB flash
cp espflash_ota_8mb.toml espflash.toml

# For 16MB flash
cp espflash_ota_16mb.toml espflash.toml
```

2. Or use the --partition-table flag:
```bash
# For 8MB flash
./build.sh run esp32s3 --flash-size 8mb

# For 16MB flash
./build.sh run esp32s3 --flash-size 16mb
```

## Development

1. Make sure you have all prerequisites installed
2. Connect your ESP32 via USB
3. Use `./build.sh run esp32s3 --watch` to build, flash and watch logs
4. Use `./build.sh watch` to just watch logs without building/flashing
