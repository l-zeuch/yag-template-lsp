use serde::Deserialize;
use tower_lsp_server::LanguageServer;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub extra_funcs: Vec<String>,
}

pub async fn handle_did_change_configuration(srv: impl LanguageServer) {}
