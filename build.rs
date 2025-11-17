use std::env;
use std::path::Path;

fn main() {
    if let Ok(sdk_path) = env::var("NPCAP_SDK_PATH") {
        println!("cargo:rustc-link-search=native={}\\Lib\\x64", sdk_path);
        println!("cargo:rustc-link-lib=static=Packet");
    } else {
        let default_path = "C:\\Program Files\\Npcap";
        if Path::new(default_path).exists() {
            println!("cargo:rustc-link-search=native={}\\Lib\\x64", default_path);
            println!("cargo:rustc-link-lib=static=Packet");
        } else {
            println!("cargo:warning=NPCAP_SDK_PATH environment variable not set and Npcap SDK not found in default location.");
            println!("cargo:warning=Please set NPCAP_SDK_PATH to the path of the Npcap SDK.");
            println!("cargo:warning=For example: NPCAP_SDK_PATH=C:\\Program Files\\Npcap");
        }
    }
}