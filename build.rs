extern crate protoc_rust;

use protoc_rust::Customize;
use std::fs;

const PROTOBUF_SOURCES: &str = "proto";

fn main() {
    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/proto",
        input: &["proto/hpv.proto"],
        includes: &["proto"],
        customize: Customize {
            ..Default::default()
        },
    }).expect("protoc");
}
