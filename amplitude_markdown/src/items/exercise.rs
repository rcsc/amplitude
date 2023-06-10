use std::{collections::HashMap, str::FromStr};

use amplitude_runner::lang::Language;

use crate::parse::parse_md;
use amplitude_runner::var_type::VariableType;

use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionConfig {
    inputs: Vec<VariableType>,
    output: VariableType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExerciseConfig {
    title: String,
    #[serde(default)]
    instructions: String,
    #[serde(flatten)]
    functions: HashMap<String, FunctionConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(into = "ExcerciseSerialize")]
pub struct Exercise {
    config: ExerciseConfig,
    code: HashMap<Language, String>,
}

impl Into<ExcerciseSerialize> for Exercise {
    fn into(self) -> ExcerciseSerialize {
        ExcerciseSerialize {
            title: self.config.title,
            instructions: self.config.instructions,
            code: self.code,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExcerciseSerialize {
    title: String,
    instructions: String,
    code: HashMap<Language, String>,
}

impl Exercise {
    pub fn from_raw(
        contents: &DirContents,
        path: &Path,
        ctx: &mut DataContext,
        id: &str,
    ) -> anyhow::Result<Self> {
        let config =
            &fs::read_to_string(path.join("config.toml")).context("While reading `config.toml`")?;
        let mut config: ExerciseConfig =
            toml::from_str(config).context("While parsing `config.toml`")?;

        let code = contents
            .query(id, FileType::Code)
            .map(|item| {
                (
                    Language::from_str(&item.ext)
                        .expect("Already guaranteed by check in Item impl of Exercise"),
                    fs::read_to_string(item.path(path)).expect("guaranteed valid path"),
                )
            })
            .collect();
        config.instructions = parse_md(&fs::read_to_string(path.join("instructions.md"))?, ctx)?;

        Ok(Self { config, code })
    }
}

impl Item for Exercise {
    fn parse_from_dir(
        dir: &Path,
        contents: DirContents,
        ctx: &mut DataContext,
    ) -> anyhow::Result<ItemType> {
        ensure!(contents.contains("instructions.md"), "instructions.md");
        ensure!(contents.contains("config.toml"), "config.toml");
        ensure!(
            contents.query("src", FileType::Directory).next().is_some(),
            "src/",
            "Source directory"
        );
        
        let id = ctx.id().rsplit('/').next().unwrap().to_string();
        ensure!(
            contents
                .query(&("src/".to_string() + &id), FileType::Code)
                .next()
                .is_some(),
            format!("src/{}.<code>", id),
            "Starting code"
        );
        ensure!(
            contents.query("src/generator", FileType::Code).next().is_some(),
            "src/generator.<code>",
            "Test Case generator"
        );

        let exercise = Exercise::from_raw(&contents, dir, ctx, &id)?;
        Ok(ItemType::Exercise(exercise))
    }
}
