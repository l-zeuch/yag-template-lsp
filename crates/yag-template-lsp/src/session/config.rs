use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use tokio::fs;
use tower_lsp_server::ls_types::{ConfigurationItem, MessageType};
use yag_template_envdefs::{EnvDefSource, EnvDefs, bundled_envdefs};

use crate::session::Session;

pub(crate) const YAG_LSP_SECTION_NAME: &str = "yagTemplate";

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) extra_envdef_files: Vec<String>,
}

impl Session {
    pub(crate) async fn reload_config(&self) {
        let response = match self
            .client
            .configuration(vec![ConfigurationItem {
                scope_uri: None,
                section: Some(YAG_LSP_SECTION_NAME.into()),
            }])
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::error!("workspace/configuration request failed: {err}");
                return;
            }
        };

        // The client sends null (or nothing at all) if the section is unset.
        let raw_config = response.into_iter().next().unwrap_or(Value::Null);
        let config: Config = match serde_json::from_value::<Option<Config>>(raw_config) {
            Ok(config) => config.unwrap_or_default(),
            Err(err) => {
                tracing::error!("Failed parsing configuration: {err}");
                self.client
                    .show_message(MessageType::ERROR, "Invalid configuration")
                    .await;
                return;
            }
        };

        let Ok(new_envdefs) = self.try_resolve_envdefs(&config.extra_envdef_files).await else {
            return;
        };
        *self.envdefs.write().unwrap() = new_envdefs;
        self.reanalyze_documents().await;
    }

    async fn try_resolve_envdefs(&self, extra_envdef_files: &[String]) -> Result<EnvDefs, ()> {
        let paths = resolve_envdef_paths(self.workspace_root().as_deref(), extra_envdef_files);
        let extra_envdefs = match load_extra_envdefs(&paths).await {
            Ok(extra_envdefs) => extra_envdefs,
            Err(err) => {
                tracing::error!("failed loading extra envdefs in config: {err}");
                self.client.show_message(MessageType::ERROR, err).await;
                return Err(());
            }
        };
        let mut new_envdefs = bundled_envdefs::load().clone();
        new_envdefs.merge(extra_envdefs);
        Ok(new_envdefs)
    }
}

fn resolve_envdef_paths(workspace_root: Option<&Path>, filenames: &[String]) -> Vec<PathBuf> {
    filenames
        .iter()
        .map(|filename| match workspace_root {
            Some(root) => root.join(filename),
            None => PathBuf::from(filename),
        })
        .collect()
}

#[derive(Debug)]
enum LoadExtraEnvdefsError {
    BadFileRead { path: PathBuf, underlying: std::io::Error },
    Syntax(yag_template_envdefs::ParseError),
}
impl fmt::Display for LoadExtraEnvdefsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LoadExtraEnvdefsError::*;
        match self {
            BadFileRead { path, underlying } => {
                write!(f, "Failed reading env def file {}: {underlying}", path.display())
            }
            Syntax(err) => write!(f, "Failed parsing env defs: {err}"),
        }
    }
}

async fn load_extra_envdefs(paths: &[PathBuf]) -> Result<EnvDefs, LoadExtraEnvdefsError> {
    let mut srcs = Vec::new();
    for path in paths {
        let contents = fs::read_to_string(path)
            .await
            .map_err(|err| LoadExtraEnvdefsError::BadFileRead {
                path: path.clone(),
                underlying: err,
            })?;
        srcs.push(EnvDefSource::new(path.display().to_string(), contents));
    }
    yag_template_envdefs::parse(&srcs).map_err(LoadExtraEnvdefsError::Syntax)
}
