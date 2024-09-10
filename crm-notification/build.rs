use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    fs::create_dir_all("src/pb")?;
    let builder = tonic_build::configure();
    builder
        .out_dir("src/pb")
        .compile(
            &[
                "../protos/notification/message.proto",
                "../protos/notification/rpc.proto",
            ],
            &[".."],
        )
        .unwrap();

    println!("cargo:rerun-if-changed=../protos/notification/message.proto");
    println!("cargo:rerun-if-changed=../protos/notification/rpc.proto");
    Ok(())
}
