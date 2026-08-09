use std::{fmt, fs};

use serde::Deserialize;
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
                tracing::error!("failed to retrieve configuration: {err}");
                return;
            }
        };

        let config: Config = response
            .first()
            .filter(|value| !value.is_null())
            .and_then(|value| {
                serde_json::from_value(value.clone())
                    .map_err(|err| {
                        tracing::warn!("failed to parse configuration, ignoring: {err}");
                    })
                    .ok()
            })
            .unwrap_or_default();

        let Ok(new_envdefs) = self.try_resolve_envdefs(&config.extra_envdef_files).await else {
            return;
        };
        *self.envdefs.write().await = new_envdefs;
    }

    async fn try_resolve_envdefs(&self, extra_envdef_files: &[String]) -> Result<EnvDefs, ()> {
        let extra_envdefs = match load_extra_envdefs(extra_envdef_files) {
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

#[derive(Debug)]
enum LoadExtraEnvdefsError {
    BadFileRead {
        filename: String,
        underlying: std::io::Error,
    },
    Syntax(yag_template_envdefs::ParseError),
}
impl fmt::Display for LoadExtraEnvdefsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LoadExtraEnvdefsError::*;
        match self {
            BadFileRead { filename, underlying } => write!(f, "Failed reading env def file {filename}: {underlying}"),
            Syntax(err) => write!(f, "Failed parsing env defs: {err}"),
        }
    }
}

fn load_extra_envdefs(filenames: &[String]) -> Result<EnvDefs, LoadExtraEnvdefsError> {
    let mut srcs = Vec::new();
    for filename in filenames {
        let contents = fs::read_to_string(filename).map_err(|err| LoadExtraEnvdefsError::BadFileRead {
            filename: filename.clone(),
            underlying: err,
        })?;
        srcs.push(EnvDefSource::new(filename, contents));
    }
    yag_template_envdefs::parse(&srcs).map_err(LoadExtraEnvdefsError::Syntax)
}
