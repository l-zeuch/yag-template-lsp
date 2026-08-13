use tower_lsp_server::ls_types::{DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams};

use crate::provider;
use crate::session::{Document, Session};

pub(crate) async fn on_document_open(sess: &Session, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let doc = Document::new(sess, uri, &params.text_document.text);
    provider::diagnostics::publish(sess, &sess.upsert_document(doc)).await;
}

pub(crate) async fn on_document_change(sess: &Session, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;

    // We're using TextDocumentSyncKind::FULL, so no incremental changes (for now.)
    let doc = Document::new(sess, uri, &params.content_changes[0].text);
    provider::diagnostics::publish(sess, &sess.upsert_document(doc)).await;
}

pub(crate) async fn on_document_close(sess: &Session, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri;
    sess.remove_document(&uri);
    provider::diagnostics::clear(sess, &uri).await;
}
