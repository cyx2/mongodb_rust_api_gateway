use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::TryStreamExt;
use mongodb::bson::Document;
use mongodb::Collection;
use tracing::instrument;

use crate::error::{ApiError, ApiResult};
use crate::models::*;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/documents/insert-one", post(insert_one))
        .route("/api/v1/documents/insert-many", post(insert_many))
        .route("/api/v1/documents/find-one", post(find_one))
        .route("/api/v1/documents/find-many", post(find_many))
        .route("/api/v1/documents/update-one", post(update_one))
        .route("/api/v1/documents/update-many", post(update_many))
        .route("/api/v1/documents/replace-one", post(replace_one))
        .route("/api/v1/documents/delete-one", post(delete_one))
        .route("/api/v1/documents/delete-many", post(delete_many))
        .route("/api/v1/collections", get(list_collections))
        .with_state(state)
}

fn map_driver_error(err: mongodb::error::Error) -> ApiError {
    ApiError::driver(format!("mongodb error: {err}"))
}

fn ensure_non_empty(namespace: &NamespacePayload) -> Result<(), ApiError> {
    if namespace.database.trim().is_empty() {
        return Err(ApiError::validation("database must be provided"));
    }
    if namespace.collection.trim().is_empty() {
        return Err(ApiError::validation("collection must be provided"));
    }
    Ok(())
}

fn collection_from_state(
    state: &AppState,
    namespace: &NamespacePayload,
) -> Result<Collection<Document>, ApiError> {
    ensure_non_empty(namespace)?;
    state.collection(namespace)
}

#[instrument(skip_all)]
async fn insert_one(
    State(state): State<AppState>,
    Json(payload): Json<InsertOneRequest>,
) -> ApiResult<Json<InsertOneResponse>> {
    let InsertOneRequest {
        namespace,
        document,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .insert_one(document, options)
        .await
        .map_err(map_driver_error)?;
    Ok(Json(InsertOneResponse {
        inserted_id: result.inserted_id,
    }))
}

#[instrument(skip_all)]
async fn insert_many(
    State(state): State<AppState>,
    Json(payload): Json<InsertManyRequest>,
) -> ApiResult<Json<InsertManyResponse>> {
    let InsertManyRequest {
        namespace,
        documents,
        options,
    } = payload;
    if documents.is_empty() {
        return Err(ApiError::validation("documents must not be empty"));
    }
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .insert_many(documents, options)
        .await
        .map_err(map_driver_error)?;
    Ok(Json(InsertManyResponse::from_result(result)))
}

#[instrument(skip_all)]
async fn find_one(
    State(state): State<AppState>,
    Json(payload): Json<FindOneRequest>,
) -> ApiResult<Json<FindOneResponse>> {
    let FindOneRequest {
        namespace,
        filter,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .find_one(filter, options)
        .await
        .map_err(map_driver_error)?;

    match result {
        Some(document) => Ok(Json(FindOneResponse { document })),
        None => Err(ApiError::not_found("document not found")),
    }
}

#[instrument(skip_all)]
async fn find_many(
    State(state): State<AppState>,
    Json(payload): Json<FindManyRequest>,
) -> ApiResult<Json<FindManyResponse>> {
    let FindManyRequest {
        namespace,
        filter,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let mut cursor = collection
        .find(filter, options)
        .await
        .map_err(map_driver_error)?;
    let mut documents = Vec::new();
    while let Some(document) = cursor.try_next().await.map_err(map_driver_error)? {
        documents.push(document);
    }
    Ok(Json(FindManyResponse { documents }))
}

#[instrument(skip_all)]
async fn update_one(
    State(state): State<AppState>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Json<UpdateResponse>> {
    let UpdateRequest {
        namespace,
        filter,
        update,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .update_one(filter, update, options.clone())
        .await
        .map_err(map_driver_error)?;
    if result.matched_count == 0
        && result.upserted_id.is_none()
        && !options.as_ref().and_then(|opt| opt.upsert).unwrap_or(false)
    {
        return Err(ApiError::not_found("no documents matched the filter"));
    }
    Ok(Json(UpdateResponse::from_update_result(result)))
}

#[instrument(skip_all)]
async fn update_many(
    State(state): State<AppState>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Json<UpdateResponse>> {
    let UpdateRequest {
        namespace,
        filter,
        update,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .update_many(filter, update, options)
        .await
        .map_err(map_driver_error)?;
    Ok(Json(UpdateResponse::from_update_result(result)))
}

#[instrument(skip_all)]
async fn replace_one(
    State(state): State<AppState>,
    Json(payload): Json<ReplaceOneRequest>,
) -> ApiResult<Json<UpdateResponse>> {
    let ReplaceOneRequest {
        namespace,
        filter,
        replacement,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .replace_one(filter, replacement, options.clone())
        .await
        .map_err(map_driver_error)?;
    if result.matched_count == 0
        && result.upserted_id.is_none()
        && !options.as_ref().and_then(|opt| opt.upsert).unwrap_or(false)
    {
        return Err(ApiError::not_found("no documents matched the filter"));
    }
    Ok(Json(UpdateResponse::from_update_result(result)))
}

#[instrument(skip_all)]
async fn delete_one(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> ApiResult<Json<DeleteResponse>> {
    let DeleteRequest {
        namespace,
        filter,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .delete_one(filter, options)
        .await
        .map_err(map_driver_error)?;
    if result.deleted_count == 0 {
        return Err(ApiError::not_found("no documents matched the filter"));
    }
    Ok(Json(DeleteResponse {
        deleted_count: result.deleted_count,
    }))
}

#[instrument(skip_all)]
async fn delete_many(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> ApiResult<Json<DeleteResponse>> {
    let DeleteRequest {
        namespace,
        filter,
        options,
    } = payload;
    let collection = collection_from_state(&state, &namespace)?;
    let result = collection
        .delete_many(filter, options)
        .await
        .map_err(map_driver_error)?;
    Ok(Json(DeleteResponse {
        deleted_count: result.deleted_count,
    }))
}

#[instrument(skip_all)]
async fn list_collections(
    State(state): State<AppState>,
    Query(query): Query<CollectionQuery>,
) -> ApiResult<Json<CollectionsResponse>> {
    if query.database.trim().is_empty() {
        return Err(ApiError::validation("database must be provided"));
    }
    let names = state
        .client
        .database(&query.database)
        .list_collection_names(None)
        .await
        .map_err(map_driver_error)?;
    Ok(Json(CollectionsResponse { collections: names }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use mongodb::Client;
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("client");
        let config = crate::config::Config {
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
        AppState::new(client, &config)
    }

    #[tokio::test]
    async fn insert_many_requires_documents() {
        let app = router(test_state().await);
        let payload = serde_json::json!({
            "database": "app",
            "collection": "users",
            "documents": []
        });
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/documents/insert-many")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
