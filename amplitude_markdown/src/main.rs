use std::{env, fs};

use amplitude_common::config::Config;
use amplitude_markdown::parse::parse;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let config = toml::from_str::<Config>(&fs::read_to_string("../config.toml")?)?;
    if env::current_dir()?.ends_with("amplitude_markdown") {
        env::set_current_dir("../")?;
    }
    parse(&config)?;

    Ok(())
}
