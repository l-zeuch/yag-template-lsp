use std::hash::RandomState;

use anyhow::Context;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use tower_lsp::Client;
use tower_lsp::lsp_types::Url;

pub(crate) mod document;
pub(crate) mod sync;

pub(crate) use document::Document;
use yag_template_envdefs::{EnvDefSource, EnvDefs, bundled_envdefs};

pub(crate) struct Session {
    pub(crate) client: Client,
    pub(crate) envdefs: tokio::sync::RwLock<EnvDefs>,
    documents: DashMap<Url, Document>,
}

impl Session {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            envdefs: tokio::sync::RwLock::new(bundled_envdefs::load().expect("bundled envdefs should be valid")),
            documents: DashMap::new(),
        }
    }

    pub(crate) fn document(&self, uri: &Url) -> anyhow::Result<Ref<'_, Url, Document, RandomState>> {
        self.documents
            .get(uri)
            .with_context(|| format!("could not find document {uri}"))
    }

    pub(crate) fn upsert_document(&self, uri: &Url, document: Document) {
        self.documents.insert(uri.clone(), document);
    }

    pub(crate) fn remove_document(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub(crate) async fn merge_custom_funcdefs(&self, custom_sources: Vec<EnvDefSource>) {
        let custom = match yag_template_envdefs::parse(&custom_sources) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(%err, "failed to parse custom envdefs");
                return;
            }
        };

        let mut envdefs = self.envdefs.write().await;
        envdefs.funcs.extend(custom.funcs);
    }
}
