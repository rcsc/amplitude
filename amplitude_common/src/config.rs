use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Default, Deserialize, Debug)]
pub struct Args {
    /// Whether or not to reclone the repo from github or to use the existing one
    #[arg(long, default_value_t = false)]
    pub local: bool,
    /// The path of the config file
    #[arg(long, default_value_t = {"config.toml".to_string()})]
    pub config: String,
}

impl Args {
    /// call `clap::Parser::parse()`
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParseConfig {
    pub git_url: String,
    pub clone_path: String,
    pub output_path: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub threads: usize,
    pub req_duration: u64,

    pub db_path: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    pub google_oauth: Option<GoogleOauth>,
    pub github_oauth: Option<GithubOauth>,
}

fn args() -> Args {
    Args::parse()
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub docker: Docker,
    #[serde(default)]
    pub auth: AuthConfig,
    pub parse: ParseConfig,
    #[serde(default = "args")]
    pub args: Args,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Docker {
    pub tmp_folder: String,
    pub command: String,
    pub timeout: u64,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct GoogleOauth {
    pub client_id: String,
    pub client_secret: String,
    pub external_url: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct GithubOauth {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LanguageConfig {
    pub name: String,
    pub path: String,
    pub image_name: String,
    pub source_path: String,
}

pub trait GetLang {
    fn get_lang(&self, lang: &str) -> Option<&LanguageConfig>;
}

impl GetLang for Vec<LanguageConfig> {
    fn get_lang(&self, lang: &str) -> Option<&LanguageConfig> {
        self.iter().find(|x| x.name == lang)
    }
}
