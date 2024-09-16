use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    fs::create_dir_all("src/pb")?;

    fs::create_dir_all("src/pb")?;
    let builder = tonic_build::configure();
    builder
        .out_dir("src/pb")
        .type_attribute(
            "user_stat.User",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute("user_stat.User", r#"#[serde(rename_all = "camelCase")]"#)
        .type_attribute("user_stat.User", "#[derive(derive_builder::Builder)]")
        .type_attribute(
            "user_stat.QueryRequest",
            "#[derive(derive_builder::Builder)]",
        )
        .type_attribute(
            "user_stat.QueryRequest",
            "#[builder(setter(into, strip_option), default)]
        ",
        )
        .type_attribute(
            "user_stat.RawQueryRequest",
            "#[derive(derive_builder::Builder)]",
        )
        .type_attribute("user_stat.TimeQuery", "#[derive(derive_builder::Builder)]")
        .type_attribute("user_stat.IdQuery", "#[derive(derive_builder::Builder)]")
        .field_attribute("user_stat.User.email", r#"#[builder(setter(into))]"#)
        .field_attribute("user_stat.User.name", r#"#[builder(setter(into))]"#)
        .field_attribute(
            "user_stat.RawQueryRequest.query",
            r#"#[builder(setter(into))]"#,
        )
        .field_attribute(
            "user_stat.TimeQuery.before",
            r#"#[builder(setter(into, strip_option))]"#,
        )
        .field_attribute(
            "user_stat.TimeQuery.after",
            r#"#[builder(setter(into, strip_option))]"#,
        )
        .field_attribute(
            "user_stat.QueryRequest.timestamps",
            r#"#[builder(setter(each(name="timestamp", into)))]"#,
        )
        .field_attribute(
            "user_stat.QueryRequest.ids",
            r#"#[builder(setter(each(name="id", into)))]"#,
        )
        .field_attribute(
            "user_stat.IdQuery.ids",
            r#"#[builder(setter(each(name="id", into)))]"#,
        )
        .compile(
            &[
                "../protos/user_stat/message.proto",
                "../protos/user_stat/rpc.proto",
            ],
            &[".."],
        )?;

    println!("cargo:rerun-if-changed=../protos/user_stat/message.proto");
    println!("cargo:rerun-if-changed=../protos/user_stat/rpc.proto");
    Ok(())
}
