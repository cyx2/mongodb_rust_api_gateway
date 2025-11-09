use dashmap::DashMap;
use mongodb::bson::Document;
use mongodb::Client;
use mongodb::Collection;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::config::Config;
use crate::error::ApiError;
use crate::models::NamespacePayload;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    client: Client,
    default_database: Option<Arc<str>>,
    default_collection: Option<Arc<str>>,
    collections: DashMap<NamespaceKey, Collection<Document>>,
}

#[derive(Clone)]
struct NamespaceKey {
    database: Arc<str>,
    collection: Arc<str>,
}

impl PartialEq for NamespaceKey {
    fn eq(&self, other: &Self) -> bool {
        self.database.as_ref() == other.database.as_ref()
            && self.collection.as_ref() == other.collection.as_ref()
    }
}

impl Eq for NamespaceKey {}

impl Hash for NamespaceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.database.as_ref().hash(state);
        self.collection.as_ref().hash(state);
    }
}

impl NamespaceKey {
    fn new(database: Arc<str>, collection: Arc<str>) -> Self {
        Self {
            database,
            collection,
        }
    }

    fn database(&self) -> &str {
        self.database.as_ref()
    }

    fn collection(&self) -> &str {
        self.collection.as_ref()
    }
}

impl AppState {
    pub fn new(client: Client, config: &Config) -> Self {
        let inner = AppStateInner {
            client,
            default_database: config.default_database.as_deref().map(Arc::<str>::from),
            default_collection: config.default_collection.as_deref().map(Arc::<str>::from),
            collections: DashMap::new(),
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn client(&self) -> &Client {
        &self.inner.client
    }

    pub fn collection(
        &self,
        namespace: &NamespacePayload,
    ) -> Result<mongodb::Collection<Document>, ApiError> {
        let resolved = self.resolve_namespace(namespace)?;
        Ok(self.inner.collection_for(&resolved))
    }

    fn resolve_namespace(&self, namespace: &NamespacePayload) -> Result<NamespaceKey, ApiError> {
        let database = match namespace.database.trim() {
            "" => self
                .inner
                .default_database
                .as_ref()
                .cloned()
                .ok_or_else(|| ApiError::validation("database must be provided"))?,
            value => Arc::<str>::from(value.to_owned()),
        };

        let collection = match namespace.collection.trim() {
            "" => self
                .inner
                .default_collection
                .as_ref()
                .cloned()
                .ok_or_else(|| ApiError::validation("collection must be provided"))?,
            value => Arc::<str>::from(value.to_owned()),
        };

        Ok(NamespaceKey::new(database, collection))
    }
}

impl AppStateInner {
    fn collection_for(&self, namespace: &NamespaceKey) -> Collection<Document> {
        if let Some(entry) = self.collections.get(namespace) {
            return entry.clone();
        }

        let collection = self
            .client
            .database(namespace.database())
            .collection::<Document>(namespace.collection());
        self.collections
            .insert(namespace.clone(), collection.clone());
        collection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collection_requires_namespace_values() {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = Config {
            mongodb_uri: "mongodb://localhost:27017".into(),
            default_database: None,
            default_collection: None,
            pool_min_size: None,
            pool_max_size: None,
            connect_timeout: None,
            server_selection_timeout: None,
            log_level: None,
            bind_address: "127.0.0.1:3000".into(),
        };
        let state = AppState::new(client, &config);
        let payload = NamespacePayload {
            database: "".into(),
            collection: "users".into(),
        };
        let err = state
            .collection(&payload)
            .expect_err("expected validation error");
        assert_eq!(err.status().as_u16(), 400);
    }

    #[tokio::test]
    async fn collection_uses_defaults_for_missing_namespace_fields() {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = Config {
            mongodb_uri: "mongodb://localhost:27017".into(),
            default_database: Some("app".into()),
            default_collection: Some("users".into()),
            pool_min_size: None,
            pool_max_size: None,
            connect_timeout: None,
            server_selection_timeout: None,
            log_level: None,
            bind_address: "127.0.0.1:3000".into(),
        };
        let state = AppState::new(client, &config);
        let payload = NamespacePayload {
            database: "   ".into(),
            collection: "   ".into(),
        };

        let collection = state.collection(&payload).expect("collection handle");
        let namespace = collection.namespace();
        assert_eq!(namespace.db, "app");
        assert_eq!(namespace.coll, "users");
    }

    #[tokio::test]
    async fn collection_caches_handles() {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = Config {
            mongodb_uri: "mongodb://localhost:27017".into(),
            default_database: None,
            default_collection: None,
            pool_min_size: None,
            pool_max_size: None,
            connect_timeout: None,
            server_selection_timeout: None,
            log_level: None,
            bind_address: "127.0.0.1:3000".into(),
        };
        let state = AppState::new(client, &config);
        let payload1 = NamespacePayload {
            database: "test_db".into(),
            collection: "test_coll".into(),
        };
        let payload2 = NamespacePayload {
            database: "test_db".into(),
            collection: "test_coll".into(),
        };

        let collection1 = state.collection(&payload1).expect("collection handle");
        let collection2 = state.collection(&payload2).expect("collection handle");

        // Both should reference the same collection
        assert_eq!(collection1.name(), collection2.name());
        assert_eq!(collection1.namespace().db, collection2.namespace().db);
    }

    #[tokio::test]
    async fn collection_handles_different_namespaces() {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = Config {
            mongodb_uri: "mongodb://localhost:27017".into(),
            default_database: None,
            default_collection: None,
            pool_min_size: None,
            pool_max_size: None,
            connect_timeout: None,
            server_selection_timeout: None,
            log_level: None,
            bind_address: "127.0.0.1:3000".into(),
        };
        let state = AppState::new(client, &config);
        let payload1 = NamespacePayload {
            database: "db1".into(),
            collection: "coll1".into(),
        };
        let payload2 = NamespacePayload {
            database: "db2".into(),
            collection: "coll2".into(),
        };

        let collection1 = state.collection(&payload1).expect("collection handle");
        let collection2 = state.collection(&payload2).expect("collection handle");

        assert_ne!(collection1.name(), collection2.name());
        assert_ne!(collection1.namespace().db, collection2.namespace().db);
    }

    #[tokio::test]
    async fn collection_trims_whitespace() {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = Config {
            mongodb_uri: "mongodb://localhost:27017".into(),
            default_database: None,
            default_collection: None,
            pool_min_size: None,
            pool_max_size: None,
            connect_timeout: None,
            server_selection_timeout: None,
            log_level: None,
            bind_address: "127.0.0.1:3000".into(),
        };
        let state = AppState::new(client, &config);
        let payload = NamespacePayload {
            database: "  test_db  ".into(),
            collection: "  test_coll  ".into(),
        };

        let collection = state.collection(&payload).expect("collection handle");
        assert_eq!(collection.name(), "test_coll");
        assert_eq!(collection.namespace().db, "test_db");
    }
}
