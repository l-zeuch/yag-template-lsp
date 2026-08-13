use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use anyhow::Context;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::Uri;

pub(crate) mod config;
pub(crate) mod document;
pub(crate) mod sync;

pub(crate) use document::Document;
use yag_template_envdefs::{EnvDefs, bundled_envdefs};

use crate::provider;

type DocumentStore = HashMap<Uri, Arc<Document>>;

pub(crate) struct Session {
    pub(crate) client: Client,
    envdefs: RwLock<EnvDefs>,
    documents: RwLock<DocumentStore>,
}

impl Session {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            envdefs: RwLock::new(bundled_envdefs::load().clone()),
            documents: RwLock::new(DocumentStore::new()),
        }
    }

    pub(crate) async fn reanalyze_documents(&self) {
        {
            let envdefs = self.read_envdefs();
            for (_, doc) in self.documents.write().unwrap().iter_mut() {
                *doc = Arc::new(doc.reanalyze_with(&envdefs))
            }
        }

        let new_documents: Vec<_> = self.documents.read().unwrap().values().map(Arc::clone).collect();
        for doc in new_documents {
            provider::diagnostics::publish(self, &doc).await;
        }
    }

    pub(crate) fn document(&self, uri: &Uri) -> anyhow::Result<Arc<Document>> {
        self.documents
            .read()
            .unwrap()
            .get(uri)
            .cloned()
            .with_context(|| format!("could not find document {uri:?}"))
    }

    pub(crate) fn upsert_document(&self, document: Document) -> Arc<Document> {
        let document = Arc::new(document);
        self.documents
            .write()
            .unwrap()
            .insert(document.uri.clone(), Arc::clone(&document));
        document
    }

    pub(crate) fn remove_document(&self, uri: &Uri) {
        self.documents.write().unwrap().remove(uri);
    }

    pub(crate) fn read_envdefs(&self) -> RwLockReadGuard<'_, EnvDefs> {
        self.envdefs.read().unwrap()
    }
}
