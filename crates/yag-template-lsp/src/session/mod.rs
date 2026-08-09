use std::hash::RandomState;

use anyhow::Context;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::Uri;

pub(crate) mod config;
pub(crate) mod document;
pub(crate) mod sync;

pub(crate) use document::Document;
use yag_template_envdefs::{EnvDefs, bundled_envdefs};

pub(crate) struct Session {
    pub(crate) client: Client,
    envdefs: tokio::sync::RwLock<EnvDefs>,
    documents: DashMap<Uri, Document>,
}

impl Session {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            envdefs: tokio::sync::RwLock::new(bundled_envdefs::load().clone()),
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

    pub(crate) async fn read_envdefs(&self) -> tokio::sync::RwLockReadGuard<'_, EnvDefs> {
        self.envdefs.read().await
    }
}
