use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{
    DeleteOptions, FindOneOptions, FindOptions, InsertManyOptions, InsertOneOptions,
    ReplaceOptions, UpdateOptions,
};
use serde::{Deserialize, Serialize};

fn empty_document() -> Document {
    doc! {}
}

#[derive(Debug, Deserialize)]
pub struct NamespacePayload {
    pub database: String,
    pub collection: String,
}

#[derive(Debug, Deserialize)]
pub struct InsertOneRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    pub document: Document,
    #[serde(default)]
    pub options: Option<InsertOneOptions>,
}

#[derive(Debug, Serialize)]
pub struct InsertOneResponse {
    pub inserted_id: Bson,
}

#[derive(Debug, Deserialize)]
pub struct InsertManyRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    pub documents: Vec<Document>,
    #[serde(default)]
    pub options: Option<InsertManyOptions>,
}

#[derive(Debug, Serialize)]
pub struct InsertManyResponse {
    pub inserted_ids: Vec<Bson>,
}

#[derive(Debug, Deserialize)]
pub struct FindOneRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    #[serde(default = "empty_document")]
    pub filter: Document,
    #[serde(default)]
    pub options: Option<FindOneOptions>,
}

#[derive(Debug, Serialize)]
pub struct FindOneResponse {
    pub document: Document,
}

#[derive(Debug, Deserialize)]
pub struct FindManyRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    #[serde(default = "empty_document")]
    pub filter: Document,
    #[serde(default)]
    pub options: Option<FindOptions>,
}

#[derive(Debug, Serialize)]
pub struct FindManyResponse {
    pub documents: Vec<Document>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    pub filter: Document,
    pub update: Document,
    #[serde(default)]
    pub options: Option<UpdateOptions>,
}

#[derive(Debug, Serialize)]
pub struct UpdateResponse {
    pub matched_count: u64,
    pub modified_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upserted_id: Option<Bson>,
}

#[derive(Debug, Deserialize)]
pub struct ReplaceOneRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    pub filter: Document,
    pub replacement: Document,
    #[serde(default)]
    pub options: Option<ReplaceOptions>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    #[serde(flatten)]
    pub namespace: NamespacePayload,
    pub filter: Document,
    #[serde(default)]
    pub options: Option<DeleteOptions>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct CollectionQuery {
    pub database: String,
}

#[derive(Debug, Serialize)]
pub struct CollectionsResponse {
    pub collections: Vec<String>,
}

impl UpdateResponse {
    pub fn from_update_result(result: mongodb::results::UpdateResult) -> Self {
        Self::from_parts(
            result.matched_count,
            result.modified_count,
            result.upserted_id,
        )
    }

    fn from_parts(matched_count: u64, modified_count: u64, upserted_id: Option<Bson>) -> Self {
        Self {
            matched_count,
            modified_count,
            upserted_id,
        }
    }
}

impl InsertManyResponse {
    pub fn from_result(result: mongodb::results::InsertManyResult) -> Self {
        Self::from_inserted_ids(result.inserted_ids)
    }

    fn from_inserted_ids(inserted_ids: std::collections::HashMap<usize, Bson>) -> Self {
        let mut ids: Vec<(usize, Bson)> = inserted_ids.into_iter().collect();
        ids.sort_by_key(|(index, _)| *index);
        Self {
            inserted_ids: ids.into_iter().map(|(_, id)| id).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::Bson;
    use std::collections::HashMap;

    #[test]
    fn update_response_reflects_update_result_fields() {
        let response = UpdateResponse::from_parts(3, 2, Some(Bson::Int32(42)));

        assert_eq!(response.matched_count, 3);
        assert_eq!(response.modified_count, 2);
        assert_eq!(response.upserted_id, Some(Bson::Int32(42)));
    }

    #[test]
    fn insert_many_response_sorts_inserted_ids() {
        let mut inserted_ids: HashMap<usize, Bson> = HashMap::new();
        inserted_ids.insert(2, Bson::Int32(2));
        inserted_ids.insert(0, Bson::Int32(0));
        inserted_ids.insert(1, Bson::Int32(1));

        let response = InsertManyResponse::from_inserted_ids(inserted_ids);

        assert_eq!(
            response.inserted_ids,
            vec![Bson::Int32(0), Bson::Int32(1), Bson::Int32(2)]
        );
    }
}
