# E-Chess

This project is a physical chess board that is connected to an online chess
server (currently lichess).\
It allows to play chess on a physical board with a computer/remote opponent.

It is currently **WIP** so no full documentation and information is available
yet.\
However there exists already a working prototype including:

- ESP32-S3 Rust firmware
- Android app
- connection to lichess (to lichess via the android app using BLE - web on the
  esp32s3 was too unstable and slow)
- LEDs in the board to show the current game state
- 3d printable OpenSCAD file
- KiCAD electronics schematic

## Future plans

- more gameplay features
- ble over secure connection (currently unencrypted!)
- more easy hardware design (maybe PCB for the whole board?)
- Documentation
- ...

## AI Disclaimer

Yes, I use some AI for this project (especially for the Android app, since I am
not that interested in writing apps...), but the code-architecture, most
firmware code, scad, electronics and the overall design is my own.
