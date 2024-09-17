use anyhow::bail;
use serde::Deserialize;
use std::{env, fs::File};

const CONFIG_FILE: &str = "CONFIG_FILE";

/// Extension methods for "configuration structs" which can be deserialized.
pub trait ConfigExt
where
    Self: for<'de> Deserialize<'de>,
{
    /// Load the configuration from the file at the value of `app.yml` or the `CONFIG_FILE` environment variable
    fn load() -> anyhow::Result<Self> {
        // prioritize local file, then system file, then env variable
        let ret = match (
            File::open("app.yml"),
            File::open("/etc/config/crm.yml"),
            env::var(CONFIG_FILE),
        ) {
            (Ok(reader), _, _) => serde_yaml::from_reader(reader),
            (_, Ok(reader), _) => serde_yaml::from_reader(reader),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => bail!("Config file not found"),
        };
        Ok(ret?)
    }
}

impl<T> ConfigExt for T where T: for<'de> Deserialize<'de> {}
