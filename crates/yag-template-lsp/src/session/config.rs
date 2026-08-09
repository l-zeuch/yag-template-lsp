use serde::Deserialize;
use tower_lsp_server::ls_types::{ConfigurationItem, MessageType};
use yag_template_envdefs::{EnvDefSource, bundled_envdefs};

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

        let cfg: Config = response
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

        self.update_envdefs(&cfg.extra_envdef_files).await;
    }

    async fn update_envdefs(&self, extra_funcs: &[String]) {
        // Obtain a fresh bundle; we may have changed workspaces with different custom envdefs,
        // so we should discard the old ones.
        let mut envdefs = bundled_envdefs::load().expect("bundled envdefs should be valid");

        for file in extra_funcs {
            let Ok(src) = EnvDefSource::new_from_file(file) else {
                tracing::warn!(path = %file, "failed to load env def");

                self.client
                    .show_message(MessageType::WARNING, format!("failed to load env def {file}, ignoring"))
                    .await;

                continue;
            };

            if let Err(err) = envdefs.extend_from_source(&src) {
                tracing::warn!(path = %file, "failed to parse env def: {err}");

                self.client
                    .show_message(
                        MessageType::WARNING,
                        format!("failed to parse env def {file}, ignoring"),
                    )
                    .await;
            }
        }

        *self.envdefs.write().await = envdefs;
    }
}
