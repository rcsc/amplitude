use anyhow::Context;

use serde::{Deserialize, Serialize};

use std::fs;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::course::Course;

pub mod article;
pub mod quiz;

/// Stores the state of the parsing process.
#[derive(Debug, Default)]
pub struct ParseState {
    pub options: comrak::ComrakOptions,
    quizzes: HashMap<(String, String), quiz::Quiz>,
    articles: HashMap<String, PathBuf>,
    pub courses: HashMap<String, Course>,
    article_configs: HashMap<String, article::ArticleConfig>,
}

impl ParseState {
    /// Get a quiz config by its id
    pub fn get_quiz(&self, article_id: &str, quiz_id: &str) -> Option<&quiz::Quiz> {
        self.quizzes.get(&(article_id.to_string(), quiz_id.to_string()))
    }

    /// Insert a quiz config into the state
    pub fn insert_quiz(&mut self, article: &str, id: &str, quiz: quiz::Quiz) -> Option<quiz::Quiz> {
        self.quizzes
            .insert((article.to_string(), id.to_string()), quiz)
    }

    /// Get a course by its id
    pub fn get_article_config(&self, article_id: &str) -> Option<&article::ArticleConfig> {
        self.article_configs.get(article_id)
    }

    /// Check of an article exists
    pub fn has_id(&self, article_id: &str) -> bool {
        self.article_configs.contains_key(article_id)
    }

    /// Get a course by its id
    pub fn insert_article(&mut self, config: article::ArticleConfig, path: &Path) {
        self.articles
            .insert(config.id.to_string(), path.to_path_buf());
        self.article_configs.insert(config.id.to_string(), config);
    }

    /// Read an article given an id
    pub fn read_article(&self, article_id: &str) -> anyhow::Result<String> {
        for c in article_id.as_bytes() {
            if c.is_ascii_alphanumeric() || b"-_".contains(c) {
                continue;
            }
            anyhow::bail!("Invalid character in path: {}", *c as char);
        }

        let path = self
            .articles
            .get(article_id)
            .context("Article id not found")?;

        std::fs::read_to_string(path).with_context(|| format!("While reading file {path:?}"))
    }
}
