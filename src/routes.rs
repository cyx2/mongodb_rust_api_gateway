use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::TryStreamExt;
use mongodb::bson::Document;
use mongodb::Collection;
use tracing::instrument;

use crate::error::{ApiError, ApiResult};
use crate::models::*;
use crate::state::AppState;

const INSERT_ONE_PATH: &str = "/api/v1/documents/insert-one";
const INSERT_MANY_PATH: &str = "/api/v1/documents/insert-many";
const FIND_ONE_PATH: &str = "/api/v1/documents/find-one";
const FIND_MANY_PATH: &str = "/api/v1/documents/find-many";
const UPDATE_ONE_PATH: &str = "/api/v1/documents/update-one";
const UPDATE_MANY_PATH: &str = "/api/v1/documents/update-many";
const REPLACE_ONE_PATH: &str = "/api/v1/documents/replace-one";
const DELETE_ONE_PATH: &str = "/api/v1/documents/delete-one";
const DELETE_MANY_PATH: &str = "/api/v1/documents/delete-many";
const LIST_COLLECTIONS_PATH: &str = "/api/v1/collections";

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(INSERT_ONE_PATH, post(insert_one))
        .route(INSERT_MANY_PATH, post(insert_many))
        .route(FIND_ONE_PATH, post(find_one))
        .route(FIND_MANY_PATH, post(find_many))
        .route(UPDATE_ONE_PATH, post(update_one))
        .route(UPDATE_MANY_PATH, post(update_many))
        .route(REPLACE_ONE_PATH, post(replace_one))
        .route(DELETE_ONE_PATH, post(delete_one))
        .route(DELETE_MANY_PATH, post(delete_many))
        .route(LIST_COLLECTIONS_PATH, get(list_collections))
        .with_state(state)
}

fn namespace_fields(namespace: &NamespacePayload) -> (&str, &str) {
    (namespace.database.trim(), namespace.collection.trim())
}

fn log_namespace_received(
    endpoint: &str,
    namespace: &NamespacePayload,
    payload_items: Option<usize>,
) {
    let (database, collection) = namespace_fields(namespace);
    match payload_items {
        Some(count) => tracing::info!(
            target = "http",
            endpoint,
            database = %database,
            collection = %collection,
            payload_items = count as u64,
            "received request"
        ),
        None => tracing::info!(
            target = "http",
            endpoint,
            database = %database,
            collection = %collection,
            "received request"
        ),
    }
}

fn log_namespace_success(
    endpoint: &str,
    namespace: &NamespacePayload,
    status: StatusCode,
    affected: Option<u64>,
) {
    let (database, collection) = namespace_fields(namespace);
    match affected {
        Some(count) => tracing::info!(
            target = "http",
            endpoint,
            database = %database,
            collection = %collection,
            status = %status,
            affected = count,
            "request completed"
        ),
        None => tracing::info!(
            target = "http",
            endpoint,
            database = %database,
            collection = %collection,
            status = %status,
            "request completed"
        ),
    }
}

fn log_request_failure(
    endpoint: &str,
    namespace: Option<&NamespacePayload>,
    error: ApiError,
) -> ApiError {
    if let Some(namespace) = namespace {
        let (database, collection) = namespace_fields(namespace);
        tracing::warn!(
            target = "http",
            endpoint,
            database = %database,
            collection = %collection,
            status = %error.status(),
            error = ?error,
            "request failed"
        );
    } else {
        tracing::warn!(
            target = "http",
            endpoint,
            status = %error.status(),
            error = ?error,
            "request failed"
        );
    }
    error
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
    log_namespace_received(INSERT_ONE_PATH, &namespace, Some(1));
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(INSERT_ONE_PATH, Some(&namespace), err))?;
    let result = collection
        .insert_one(document, options)
        .await
        .map_err(|err| {
            log_request_failure(INSERT_ONE_PATH, Some(&namespace), map_driver_error(err))
        })?;
    let response = Json(InsertOneResponse {
        inserted_id: result.inserted_id,
    });
    log_namespace_success(INSERT_ONE_PATH, &namespace, StatusCode::OK, Some(1));
    Ok(response)
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
    log_namespace_received(INSERT_MANY_PATH, &namespace, Some(documents.len()));
    if documents.is_empty() {
        return Err(log_request_failure(
            INSERT_MANY_PATH,
            Some(&namespace),
            ApiError::validation("documents must not be empty"),
        ));
    }
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(INSERT_MANY_PATH, Some(&namespace), err))?;
    let result = collection
        .insert_many(documents, options)
        .await
        .map_err(|err| {
            log_request_failure(INSERT_MANY_PATH, Some(&namespace), map_driver_error(err))
        })?;
    let response = Json(InsertManyResponse::from_result(result));
    log_namespace_success(
        INSERT_MANY_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.inserted_ids.len() as u64),
    );
    Ok(response)
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
    log_namespace_received(FIND_ONE_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(FIND_ONE_PATH, Some(&namespace), err))?;
    let result = collection.find_one(filter, options).await.map_err(|err| {
        log_request_failure(FIND_ONE_PATH, Some(&namespace), map_driver_error(err))
    })?;

    match result {
        Some(document) => {
            let response = Json(FindOneResponse { document });
            log_namespace_success(FIND_ONE_PATH, &namespace, StatusCode::OK, Some(1));
            Ok(response)
        }
        None => Err(log_request_failure(
            FIND_ONE_PATH,
            Some(&namespace),
            ApiError::not_found("document not found"),
        )),
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
    log_namespace_received(FIND_MANY_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(FIND_MANY_PATH, Some(&namespace), err))?;
    let mut cursor = collection.find(filter, options).await.map_err(|err| {
        log_request_failure(FIND_MANY_PATH, Some(&namespace), map_driver_error(err))
    })?;
    let mut documents = Vec::new();
    while let Some(document) = cursor.try_next().await.map_err(|err| {
        log_request_failure(FIND_MANY_PATH, Some(&namespace), map_driver_error(err))
    })? {
        documents.push(document);
    }
    let response = Json(FindManyResponse { documents });
    let count = response.documents.len() as u64;
    log_namespace_success(FIND_MANY_PATH, &namespace, StatusCode::OK, Some(count));
    Ok(response)
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
    log_namespace_received(UPDATE_ONE_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(UPDATE_ONE_PATH, Some(&namespace), err))?;
    let result = collection
        .update_one(filter, update, options.clone())
        .await
        .map_err(|err| {
            log_request_failure(UPDATE_ONE_PATH, Some(&namespace), map_driver_error(err))
        })?;
    if result.matched_count == 0
        && result.upserted_id.is_none()
        && !options.as_ref().and_then(|opt| opt.upsert).unwrap_or(false)
    {
        return Err(log_request_failure(
            UPDATE_ONE_PATH,
            Some(&namespace),
            ApiError::not_found("no documents matched the filter"),
        ));
    }
    let response = Json(UpdateResponse::from_update_result(result));
    log_namespace_success(
        UPDATE_ONE_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.modified_count),
    );
    Ok(response)
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
    log_namespace_received(UPDATE_MANY_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(UPDATE_MANY_PATH, Some(&namespace), err))?;
    let result = collection
        .update_many(filter, update, options)
        .await
        .map_err(|err| {
            log_request_failure(UPDATE_MANY_PATH, Some(&namespace), map_driver_error(err))
        })?;
    let response = Json(UpdateResponse::from_update_result(result));
    log_namespace_success(
        UPDATE_MANY_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.modified_count),
    );
    Ok(response)
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
    log_namespace_received(REPLACE_ONE_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(REPLACE_ONE_PATH, Some(&namespace), err))?;
    let result = collection
        .replace_one(filter, replacement, options.clone())
        .await
        .map_err(|err| {
            log_request_failure(REPLACE_ONE_PATH, Some(&namespace), map_driver_error(err))
        })?;
    if result.matched_count == 0
        && result.upserted_id.is_none()
        && !options.as_ref().and_then(|opt| opt.upsert).unwrap_or(false)
    {
        return Err(log_request_failure(
            REPLACE_ONE_PATH,
            Some(&namespace),
            ApiError::not_found("no documents matched the filter"),
        ));
    }
    let response = Json(UpdateResponse::from_update_result(result));
    log_namespace_success(
        REPLACE_ONE_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.modified_count),
    );
    Ok(response)
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
    log_namespace_received(DELETE_ONE_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(DELETE_ONE_PATH, Some(&namespace), err))?;
    let result = collection
        .delete_one(filter, options)
        .await
        .map_err(|err| {
            log_request_failure(DELETE_ONE_PATH, Some(&namespace), map_driver_error(err))
        })?;
    if result.deleted_count == 0 {
        return Err(log_request_failure(
            DELETE_ONE_PATH,
            Some(&namespace),
            ApiError::not_found("no documents matched the filter"),
        ));
    }
    let response = Json(DeleteResponse {
        deleted_count: result.deleted_count,
    });
    log_namespace_success(
        DELETE_ONE_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.deleted_count),
    );
    Ok(response)
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
    log_namespace_received(DELETE_MANY_PATH, &namespace, None);
    let collection = collection_from_state(&state, &namespace)
        .map_err(|err| log_request_failure(DELETE_MANY_PATH, Some(&namespace), err))?;
    let result = collection
        .delete_many(filter, options)
        .await
        .map_err(|err| {
            log_request_failure(DELETE_MANY_PATH, Some(&namespace), map_driver_error(err))
        })?;
    let response = Json(DeleteResponse {
        deleted_count: result.deleted_count,
    });
    log_namespace_success(
        DELETE_MANY_PATH,
        &namespace,
        StatusCode::OK,
        Some(response.deleted_count),
    );
    Ok(response)
}

#[instrument(skip_all)]
async fn list_collections(
    State(state): State<AppState>,
    Query(query): Query<CollectionQuery>,
) -> ApiResult<Json<CollectionsResponse>> {
    tracing::info!(
        target = "http",
        endpoint = LIST_COLLECTIONS_PATH,
        database = %query.database.trim(),
        "received request"
    );
    if query.database.trim().is_empty() {
        return Err(log_request_failure(
            LIST_COLLECTIONS_PATH,
            None,
            ApiError::validation("database must be provided"),
        ));
    }
    let names = state
        .client()
        .database(&query.database)
        .list_collection_names(None)
        .await
        .map_err(|err| log_request_failure(LIST_COLLECTIONS_PATH, None, map_driver_error(err)))?;
    let response = Json(CollectionsResponse { collections: names });
    tracing::info!(
        target = "http",
        endpoint = LIST_COLLECTIONS_PATH,
        database = %query.database.trim(),
        status = %StatusCode::OK,
        collections = response.collections.len() as u64,
        "request completed"
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use mongodb::Client;
    use tower::ServiceExt;

    fn namespace(database: &str, collection: &str) -> NamespacePayload {
        NamespacePayload {
            database: database.into(),
            collection: collection.into(),
        }
    }

    #[test]
    fn ensure_non_empty_accepts_populated_namespace() {
        let payload = namespace("app", "users");
        assert!(ensure_non_empty(&payload).is_ok());
    }

    #[test]
    fn ensure_non_empty_rejects_blank_database() {
        let payload = namespace("   ", "users");
        let err = ensure_non_empty(&payload).expect_err("expected validation error");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn ensure_non_empty_rejects_blank_collection() {
        let payload = namespace("app", "   ");
        let err = ensure_non_empty(&payload).expect_err("expected validation error");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn log_request_failure_preserves_error() {
        let error = ApiError::validation("oops");
        let status = error.status();
        let returned = log_request_failure("/test", None, error);
        assert_eq!(returned.status(), status);
    }

    #[test]
    fn map_driver_error_wraps_mongodb_errors() {
        let driver_error = mongodb::error::Error::custom("driver boom");
        let api_error = map_driver_error(driver_error);
        assert_eq!(api_error.status(), StatusCode::BAD_GATEWAY);
    }

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
    async fn collection_from_state_requires_database() {
        let state = test_state().await;
        let payload = namespace("   ", "users");
        let err =
            collection_from_state(&state, &payload).expect_err("expected missing database error");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn collection_from_state_returns_collection_handle() {
        let state = test_state().await;
        let payload = namespace("app", "users");
        let collection = collection_from_state(&state, &payload).expect("collection handle");
        assert_eq!(collection.name(), "users");
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

    #[tokio::test]
    async fn namespace_fields_trims_whitespace() {
        let payload = namespace("  db  ", "  coll  ");
        let (db, coll) = namespace_fields(&payload);
        assert_eq!(db, "db");
        assert_eq!(coll, "coll");
    }

    #[test]
    fn collection_from_state_validates_namespace() {
        // This test verifies the validation logic
        let payload = namespace("", "users");
        // The actual validation happens in ensure_non_empty
        let err = ensure_non_empty(&payload).expect_err("expected validation error");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }
}
