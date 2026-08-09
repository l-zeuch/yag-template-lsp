use serde::Deserialize;
use tower_lsp_server::ls_types::ConfigurationItem;

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

        self.update_envdefs(cfg.extra_envdef_files).await;
    }
}
