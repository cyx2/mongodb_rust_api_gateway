use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use futures::TryStreamExt;
use mongodb::{
    Client, Database,
    bson::{Bson, Document, doc, oid::ObjectId},
    options::{FindOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr};
use thiserror::Error;
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("mongodb error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("serialization error: {0}")]
    BsonSer(#[from] mongodb::bson::ser::Error),
    #[error("deserialization error: {0}")]
    BsonDe(#[from] mongodb::bson::de::Error),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("missing configuration: {0}")]
    MissingConfig(&'static str),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::Mongo(_) | AppError::BsonSer(_) | AppError::BsonDe(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::InvalidIdentifier(_) => StatusCode::BAD_REQUEST,
            AppError::MissingConfig(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(serde_json::json!({
            "error": self.to_string(),
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct MutationResponse {
    matched_count: u64,
    modified_count: u64,
}

#[derive(Debug, Serialize)]
struct DeleteResponse {
    deleted_count: u64,
}

#[derive(Debug, Serialize)]
struct InsertResponse {
    inserted_id: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PaginationOptions {
    limit: Option<i64>,
    skip: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let mongodb_uri =
        env::var("MONGODB_URI").map_err(|_| AppError::MissingConfig("MONGODB_URI"))?;
    let client = Client::with_uri_str(&mongodb_uri).await?;
    let db = client.default_database().ok_or(AppError::MissingConfig(
        "default database name in MONGODB_URI",
    ))?;

    let collections = db.list_collection_names(None).await?;
    info!(collections = ?collections, "Discovered MongoDB collections");

    let state = AppState { db };

    let app = Router::new()
        .route("/health", get(health))
        .route("/collections", get(list_collections))
        .route(
            "/collections/:collection",
            get(list_documents).post(create_document),
        )
        .route(
            "/collections/:collection/:id",
            get(get_document)
                .put(update_document)
                .delete(delete_document),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(%addr, "Starting API gateway");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(AppError::from)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_collections(State(state): State<AppState>) -> Result<Json<Vec<String>>, AppError> {
    let names = state.db.list_collection_names(None).await?;
    Ok(Json(names))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Query(pagination): Query<PaginationOptions>,
) -> Result<Json<Vec<Document>>, AppError> {
    let collection_ref = state.db.collection::<Document>(&collection);

    let mut find_options = FindOptions::default();
    if let Some(limit) = pagination.limit {
        find_options.limit = Some(limit);
    }
    if let Some(skip) = pagination.skip {
        find_options.skip = Some(skip);
    }

    let mut cursor = collection_ref.find(None, find_options).await?;
    let mut results = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        results.push(doc);
    }

    Ok(Json(results))
}

async fn get_document(
    State(state): State<AppState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<Option<Document>>, AppError> {
    let collection_ref = state.db.collection::<Document>(&collection);
    let filter = doc! {"_id": parse_identifier(&id)?};
    let doc = collection_ref.find_one(filter, None).await?;
    Ok(Json(doc))
}

async fn create_document(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<InsertResponse>, AppError> {
    let collection_ref = state.db.collection::<Document>(&collection);
    let document = mongodb::bson::to_document(&payload)?;
    let result = collection_ref.insert_one(document, None).await?;

    let inserted_id = bson_value_to_json(result.inserted_id);
    Ok(Json(InsertResponse { inserted_id }))
}

async fn update_document(
    State(state): State<AppState>,
    Path((collection, id)): Path<(String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<MutationResponse>, AppError> {
    let collection_ref = state.db.collection::<Document>(&collection);
    let mut update_doc = mongodb::bson::to_document(&payload)?;
    update_doc.remove("_id");

    let filter = doc! {"_id": parse_identifier(&id)?};
    let update = doc! {"$set": update_doc};
    let options = UpdateOptions::builder().upsert(false).build();
    let result = collection_ref.update_one(filter, update, options).await?;

    Ok(Json(MutationResponse {
        matched_count: result.matched_count,
        modified_count: result.modified_count,
    }))
}

async fn delete_document(
    State(state): State<AppState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, AppError> {
    let collection_ref = state.db.collection::<Document>(&collection);
    let filter = doc! {"_id": parse_identifier(&id)?};
    let result = collection_ref.delete_one(filter, None).await?;
    Ok(Json(DeleteResponse {
        deleted_count: result.deleted_count,
    }))
}

fn parse_identifier(id: &str) -> Result<Bson, AppError> {
    if let Ok(object_id) = ObjectId::parse_str(id) {
        Ok(Bson::ObjectId(object_id))
    } else if !id.is_empty() {
        Ok(Bson::String(id.to_string()))
    } else {
        Err(AppError::InvalidIdentifier(id.to_string()))
    }
}

fn bson_value_to_json(value: Bson) -> serde_json::Value {
    match mongodb::bson::from_bson::<serde_json::Value>(value) {
        Ok(json) => json,
        Err(err) => {
            error!(%err, "Failed to convert BSON value to JSON");
            serde_json::Value::Null
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use hyper::body::to_bytes;

    #[test]
    fn parse_identifier_accepts_object_id() {
        let object_id = ObjectId::new();
        let hex = object_id.to_hex();
        let parsed = parse_identifier(&hex).expect("object id should parse");
        match parsed {
            Bson::ObjectId(found) => assert_eq!(found, object_id),
            other => panic!("expected object id, got {:?}", other),
        }
    }

    #[test]
    fn parse_identifier_accepts_string() {
        let parsed = parse_identifier("custom-id").expect("string id should parse");
        match parsed {
            Bson::String(s) => assert_eq!(s, "custom-id"),
            other => panic!("expected string, got {:?}", other),
        }
    }

    #[test]
    fn parse_identifier_rejects_empty() {
        let err = parse_identifier("").expect_err("empty id should fail");
        assert!(matches!(err, AppError::InvalidIdentifier(_)));
    }

    #[test]
    fn bson_value_to_json_success() {
        let json = bson_value_to_json(Bson::String("value".into()));
        assert_eq!(json, serde_json::Value::String("value".into()));
    }

    #[test]
    fn bson_value_to_json_failure_returns_null() {
        let json = bson_value_to_json(Bson::Undefined);
        assert_eq!(json, serde_json::Value::Null);
    }

    #[tokio::test]
    async fn app_error_invalid_identifier_response() {
        let response = AppError::InvalidIdentifier("bad".into()).into_response();
        let status = response.status();
        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "invalid identifier: bad");
    }

    #[tokio::test]
    async fn app_error_missing_config_response() {
        let response = AppError::MissingConfig("MONGODB_URI").into_response();
        let status = response.status();
        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"], "missing configuration: MONGODB_URI");
    }
}
