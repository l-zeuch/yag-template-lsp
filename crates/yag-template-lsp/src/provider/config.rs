use serde::Deserialize;
use tower_lsp_server::ls_types::ConfigurationItem;

use crate::session::Session;

pub const YAG_LSP_SECTION_NAME: &str = "yagTemplate";

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub extra_funcs: Vec<String>,
}

pub async fn handle_did_change_configuration(sess: &Session) {
    let response = match sess
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

    sess.update_envdefs(cfg.extra_funcs).await;
}
