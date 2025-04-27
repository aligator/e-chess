Update toolchain:
espup install


## Select esp

* esp32 s3 (default)  
`cargo run --features no_board --target xtensa-esp32s3-espidf`
* esp32  
`cargo run --features no_board --target xtensa-esp32-espidf`


## OTA
For OTA you need more than 4mb flash since the firmware is already > 2mb.  
The flash must be partitioned with two slots.  
For a 16mb flash you may use this pre-configured partition table.
```
mv espflash_ota_16mb.toml espflash.toml
```