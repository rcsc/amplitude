use crate::error::HandledRoute;
use crate::error::StatusContext;
use afire::{extension::ServeStatic, prelude::*};
use amplitude_common::config;
use amplitude_state::State;
use derive_more::{Deref, DerefMut};
use serde::{de, Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

mod article;
mod quiz;

#[derive(Debug, Deref, DerefMut, Deserialize)]
pub struct ArticleReq {
    pub article: ArticlePath,
}

impl ArticlePath {
    pub fn file_path(&self) -> PathBuf {
        config::RENDERED.join(self.path.with_extension("html"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Deref, DerefMut, Serialize)]
pub struct ArticlePath {
    pub path: PathBuf,
}

impl<'de> Deserialize<'de> for ArticlePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let path = PathBuf::from(String::deserialize(deserializer)?);
        for c in path.components() {
            match c {
                Component::CurDir => {}
                Component::Normal(_) => {}
                _ => return Err(de::Error::custom(format!("Invalid path component: {c:?}",))),
            }
        }
        Ok(ArticlePath { path })
    }
}

pub fn attach(server: &mut Server<State>) {
    article::attach(server);
    quiz::attach(server);

    ServeStatic::new("web/dist")
        .not_found(|_req, _dis| {
            Response::new()
                .stream(File::open("web/dist/index.html").expect("Webpage not built"))
                .content(Content::HTML)
        })
        .attach(server);
}