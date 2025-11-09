use std::borrow::Cow;

use mongodb::bson::Document;
use mongodb::Client;

use crate::config::Config;
use crate::error::ApiError;
use crate::models::NamespacePayload;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub default_database: Option<String>,
    pub default_collection: Option<String>,
}

impl AppState {
    pub fn new(client: Client, config: &Config) -> Self {
        Self {
            client,
            default_database: config.default_database.clone(),
            default_collection: config.default_collection.clone(),
        }
    }

    pub fn collection(
        &self,
        namespace: &NamespacePayload,
    ) -> Result<mongodb::Collection<Document>, ApiError> {
        let database: Cow<'_, str> = if namespace.database.trim().is_empty() {
            Cow::Owned(
                self.default_database
                    .clone()
                    .ok_or_else(|| ApiError::validation("database must be provided"))?,
            )
        } else {
            Cow::Borrowed(namespace.database.trim())
        };

        let collection: Cow<'_, str> = if namespace.collection.trim().is_empty() {
            Cow::Owned(
                self.default_collection
                    .clone()
                    .ok_or_else(|| ApiError::validation("collection must be provided"))?,
            )
        } else {
            Cow::Borrowed(namespace.collection.trim())
        };

        Ok(self
            .client
            .database(&database)
            .collection::<Document>(&collection))
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
}
