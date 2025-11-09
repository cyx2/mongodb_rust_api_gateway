mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hello_rust::routes;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_insert_one_and_find_one() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert a document
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "document": {
            "name": "test_user",
            "email": "test@example.com"
        }
    });

    let insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(insert_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(insert_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // inserted_id can be a string or an ObjectId object
    assert!(
        response["inserted_id"].is_string()
            || (response["inserted_id"].is_object() && response["inserted_id"]["$oid"].is_string())
    );

    // Find the document
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "email": "test@example.com"
        }
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["document"]["name"], "test_user");
    assert_eq!(response["document"]["email"], "test@example.com");
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_insert_many_and_find_many() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert multiple documents
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "documents": [
            { "name": "user1", "value": 1 },
            { "name": "user2", "value": 2 },
            { "name": "user3", "value": 3 }
        ]
    });

    let insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(insert_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(insert_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["inserted_ids"].as_array().unwrap().len(), 3);

    // Find all documents
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {}
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["documents"].as_array().unwrap().len(), 3);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_find_one_returns_404_when_not_found() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "nonexistent": "value"
        }
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_update_one() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert a document first
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "document": {
            "name": "original",
            "value": 10
        }
    });

    let insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(insert_response.status(), StatusCode::OK);

    // Update the document
    let update_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "name": "original"
        },
        "update": {
            "$set": {
                "value": 20
            }
        }
    });

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/update-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(update_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["matched_count"], 1);
    assert_eq!(response["modified_count"], 1);

    // Verify the update
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "name": "original"
        }
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["document"]["value"], 20);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_update_many() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert multiple documents
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "documents": [
            { "status": "pending", "count": 1 },
            { "status": "pending", "count": 2 },
            { "status": "done", "count": 3 }
        ]
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Update many documents
    let update_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "status": "pending"
        },
        "update": {
            "$set": {
                "status": "processed"
            }
        }
    });

    let update_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/update-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(update_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["matched_count"], 2);
    assert_eq!(response["modified_count"], 2);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_replace_one() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert a document
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "document": {
            "name": "old",
            "value": 1
        }
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Replace the document
    let replace_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "name": "old"
        },
        "replacement": {
            "name": "new",
            "value": 2,
            "extra": "field"
        }
    });

    let replace_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/replace-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(replace_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(replace_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(replace_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["matched_count"], 1);
    assert_eq!(response["modified_count"], 1);

    // Verify replacement
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "name": "new"
        }
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["document"]["value"], 2);
    assert_eq!(response["document"]["extra"], "field");
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_delete_one() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert documents
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "documents": [
            { "name": "delete_me", "value": 1 },
            { "name": "keep_me", "value": 2 }
        ]
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Delete one document
    let delete_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "name": "delete_me"
        }
    });

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/delete-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(delete_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(delete_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["deleted_count"], 1);

    // Verify deletion
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {}
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["documents"].as_array().unwrap().len(), 1);
    assert_eq!(response["documents"][0]["name"], "keep_me");
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_delete_many() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert documents
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "documents": [
            { "status": "temp", "value": 1 },
            { "status": "temp", "value": 2 },
            { "status": "permanent", "value": 3 }
        ]
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Delete many documents
    let delete_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "status": "temp"
        }
    });

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/delete-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(delete_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(delete_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["deleted_count"], 2);

    // Verify deletion
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {}
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let documents = response["documents"].as_array().unwrap();
    // Verify that only documents with status "permanent" remain (may have leftovers from other tests)
    let permanent_docs: Vec<_> = documents
        .iter()
        .filter(|doc| doc["status"] == "permanent")
        .collect();
    assert_eq!(permanent_docs.len(), 1);
    assert_eq!(permanent_docs[0]["status"], "permanent");
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_delete_one_returns_404_when_not_found() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    let delete_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "nonexistent": "value"
        }
    });

    let delete_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/delete-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(delete_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_update_one_returns_404_when_not_found() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    let update_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "nonexistent": "value"
        },
        "update": {
            "$set": {
                "value": 1
            }
        }
    });

    let update_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/update-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(update_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_replace_one_returns_404_when_not_found() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    let replace_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {
            "nonexistent": "value"
        },
        "replacement": {
            "value": 1
        }
    });

    let replace_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/replace-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(replace_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(replace_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_validation_errors_for_missing_database() {
    let state = common::test_state().await;
    let app = routes::router(state);

    let payload = json!({
        "database": "",
        "collection": "test",
        "document": { "value": 1 }
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
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
#[ignore] // Requires MongoDB to be running
async fn test_validation_errors_for_missing_collection() {
    let state = common::test_state().await;
    let app = routes::router(state);

    let payload = json!({
        "database": "test",
        "collection": "",
        "document": { "value": 1 }
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
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
#[ignore] // Requires MongoDB to be running
async fn test_list_collections() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();

    // Create a collection by inserting a document
    let insert_payload = json!({
        "database": db.clone(),
        "collection": "test_collection",
        "document": { "value": 1 }
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-one")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // List collections
    let list_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/collections?database={}", db))
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(response["collections"].is_array());
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_list_collections_requires_database() {
    let state = common::test_state().await;
    let app = routes::router(state);

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/collections?database=")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore] // Requires MongoDB to be running
async fn test_find_many_with_options() {
    let state = common::test_state().await;
    let app = routes::router(state);
    let db = common::unique_database();
    let coll = common::unique_collection();

    // Insert documents
    let insert_payload = json!({
        "database": db,
        "collection": coll,
        "documents": [
            { "name": "a", "value": 1 },
            { "name": "b", "value": 2 },
            { "name": "c", "value": 3 }
        ]
    });

    let _insert_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/insert-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(insert_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Find with limit
    let find_payload = json!({
        "database": db,
        "collection": coll,
        "filter": {},
        "options": {
            "limit": 2
        }
    });

    let find_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/find-many")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(find_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(find_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response["documents"].as_array().unwrap().len(), 2);
}

// Cleanup module that runs after tests
mod cleanup {
    use super::common;

    #[tokio::test]
    #[ignore] // Requires MongoDB to be running
    async fn cleanup_test_databases() {
        // Run cleanup to remove test databases created during test runs
        // Run this test manually after other tests: cargo test --test integration_test -- --ignored cleanup_test_databases
        // Or it will run automatically if included in the test run
        common::cleanup_test_databases().await;
    }
}
