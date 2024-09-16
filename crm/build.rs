use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    fs::create_dir_all("src/pb")?;
    let builder = tonic_build::configure();
    builder
        .out_dir("src/pb")
        .type_attribute("crm.WelcomeRequest", "#[derive(derive_builder::Builder)]")
        .type_attribute("crm.RecallRequest", "#[derive(derive_builder::Builder)]")
        .type_attribute("crm.RemindRequest", "#[derive(derive_builder::Builder)]")
        .field_attribute(
            "crm.WelcomeRequest.content_ids",
            r#"#[builder(setter(each(name="content_id", into)))]"#,
        )
        .compile(
            &["../protos/crm/message.proto", "../protos/crm/rpc.proto"],
            &["../protos/crm"],
        )?;
    Ok(())
}
