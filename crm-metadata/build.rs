use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    fs::create_dir_all("src/pb")?;
    let builder = tonic_build::configure();
    builder
        .out_dir("src/pb")
        .type_attribute("metadata.MaterializeRequest", r#"#[derive(Eq, Hash)]"#)
        .compile(
            &[
                "../protos/metadata/message.proto",
                "../protos/metadata/rpc.proto",
            ],
            &[".."],
        )
        .unwrap();
    Ok(())
}
