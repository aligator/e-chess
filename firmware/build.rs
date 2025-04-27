fn main() {
    println!("cargo::rustc-check-cfg=cfg(esp32s3)");
    println!("cargo::rustc-check-cfg=cfg(esp32)");
    embuild::espidf::sysenv::output();
}
