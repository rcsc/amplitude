use std::{
    collections::HashSet,
    fs,
    io::{self, BufRead, Read},
};

use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::{parse::parse_md, OsStrToString};

use super::*;

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Article {
    pub title: String,
}

impl Item for Article {
    fn parse_from_dir(dir: &Path, context: &mut ItemContext) -> anyhow::Result<ItemType>
    where
        Self: Sized,
    {
        let items = dir
            .read_dir()?
            .filter_map(|e| e.ok().map(|p| p.file_name().to_string()))
            .collect::<HashSet<_>>();
        anyhow::ensure!(
            items.contains("article.md"),
            "Required item: `article.md` not found",
        );
        anyhow::ensure!(items.len() == 1, "Unexpected files / directories",);

        let (config, s): (Article, String) = parse_frontmatter(&dir.join("article.md"))
            .context("While reading article / parsing frontmatter header")?;
        let html = parse_md(&s, context).context("While parsing article markdown")?;

        context
            .write_article(&html)
            .context("While writing article to disk")?;

        Ok(ItemType::Article(config))
    }
}

pub fn parse_frontmatter<T: DeserializeOwned>(path: &Path) -> anyhow::Result<(T, String)> {
    let file = fs::File::open(path).unwrap();
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

    anyhow::bail!(
        "Did not find end of Frontmatter header on path {}",
        path.display()
    )
}