use std::hash::RandomState;

use anyhow::Context;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageType, Uri};

pub(crate) mod config;
pub(crate) mod document;
pub(crate) mod sync;

pub(crate) use document::Document;
use yag_template_envdefs::{EnvDefSource, EnvDefs, bundled_envdefs};

pub(crate) struct Session {
    pub(crate) client: Client,
    pub(crate) envdefs: tokio::sync::RwLock<EnvDefs>,
    documents: DashMap<Uri, Document>,
}

impl Session {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            envdefs: tokio::sync::RwLock::new(bundled_envdefs::load().expect("bundled envdefs should be valid")),
            documents: DashMap::new(),
        }
    }

    pub(crate) fn document(&self, uri: &Uri) -> anyhow::Result<Ref<'_, Uri, Document, RandomState>> {
        self.documents
            .get(uri)
            .with_context(|| format!("could not find document {uri:?}"))
    }

    pub(crate) fn upsert_document(&self, uri: &Uri, document: Document) {
        self.documents.insert(uri.clone(), document);
    }

    pub(crate) fn remove_document(&self, uri: &Uri) {
        self.documents.remove(uri);
    }

    pub(crate) async fn update_envdefs(&self, extra_funcs: Vec<String>) {
        // Obtain a fresh bundle; we may have changed workspaces with different custom envdefs,
        // so we should discard the old ones.
        let mut envdefs = bundled_envdefs::load().expect("bundled envdefs should be valid");

        for file in &extra_funcs {
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
