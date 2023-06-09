use std::io::{self, BufRead, Read};

use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::parse::{inject::InjectData, parse_md_full};

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawArticle {
    pub title: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Article {
    pub title: String,
    pub body: String,
    pub inject_data: InjectData,
}

impl Article {
    pub fn from_raw(raw: RawArticle, body: String, inject_data: InjectData) -> Self {
        Self {
            title: raw.title,
            body,
            inject_data,
        }
    }
}

impl FromFile for Article {
    fn from_file(
        _: &str,
        file: &mut File,
        context: &mut DataContext,
        _: &Config,
    ) -> anyhow::Result<Self> {
        let (raw, s) = parse_frontmatter(file)
            .context("While reading article / parsing frontmatter header")?;
        let (html, data) = parse_md_full(&s, context).context("While parsing article markdown")?;

        let article = Article::from_raw(raw, html, data);
        Ok(article)
    }
}

pub fn parse_frontmatter<T: DeserializeOwned>(file: &File) -> anyhow::Result<(T, String)> {
    let mut reader = io::BufReader::new(file);
    let mut line = String::new();

    while line.trim().is_empty() {
        reader.read_line(&mut line)?;
    }
    anyhow::ensure!(
        line.trim() == "---",
        "Did not find Frontmatter header on article (Headers start with `---`)"
    );

    line = String::new();
    let mut header = String::new();

    while !matches!(reader.read_line(&mut line), Ok(0)) {
        if line.trim() == "---" {
            let config: T = toml::from_str(&header).context("while parsing frontmatter toml")?;
            let mut rest = vec![];
            reader.read_to_end(&mut rest).unwrap();
            let rest = String::from_utf8(rest).context("Invalid utf-8 in file")?;
            return Ok((config, rest));
        }

        header.push_str(&line);
        line = String::new();
    }

    anyhow::bail!("Did not find end of Frontmatter header")
}
