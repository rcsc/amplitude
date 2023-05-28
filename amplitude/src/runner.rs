use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use afire::internal::encoding::url;
use amplitude_common::{config::GetLang, lang::Language};
use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::state::State;

#[derive(Serialize, Deserialize)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub runtime: Duration,
    pub exit_code: i32,
}

pub fn run(app: Arc<State>, lang: Language, src: &str, args: &str) -> anyhow::Result<RunOutput> {
    let cfg = &app.config.docker;

    let mut code_file = tempfile::NamedTempFile::new_in(&cfg.tmp_folder).unwrap();
    code_file.write_all(src.as_bytes()).unwrap();

    let lang = app
        .language_config
        .get_lang(lang.image())
        .context("Language not loaded")?;

    let time = Instant::now();
    let run = Command::new(&cfg.command)
        .args([
            "run",
            "--rm",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--net",
            "none",
            "--memory",
            "128m",
            "--memory-swap",
            "256m",
            "--pids-limit",
            "512",
            "-v",
            &format!(
                "{}:/runner/{}",
                code_file.path().to_string_lossy(),
                lang.source_path
            ),
            "-e",
            &format!("TIMEOUT={}", &cfg.timeout),
            "-e",
            &format!("ARGS={}", url::encode(args)),
            &lang.image_name,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    Ok(RunOutput {
        stdout: String::from_utf8_lossy(&run.stdout).to_string(),
        stderr: String::from_utf8_lossy(&run.stderr).to_string(),
        runtime: time.elapsed(),
        exit_code: run.status.code().unwrap(),
    })
}
