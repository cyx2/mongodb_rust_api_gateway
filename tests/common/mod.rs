use hello_rust::config::Config;
use hello_rust::state::AppState;
use mongodb::Client;
use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

fn init_env() {
    INIT.call_once(|| {
        // Load .env file if it exists (ignore errors if it doesn't)
        dotenvy::dotenv().ok();
    });
}

pub fn mongodb_test_uri() -> String {
    init_env();
    env::var("MONGODB_TEST_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string())
}

pub async fn mongodb_available() -> bool {
    let uri = mongodb_test_uri();
    match Client::with_uri_str(&uri).await {
        Ok(client) => {
            // Try to ping the server
            match client
                .database("admin")
                .run_command(mongodb::bson::doc! { "ping": 1 }, None)
                .await
            {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

pub async fn test_state() -> AppState {
    let uri = mongodb_test_uri();
    let client = Client::with_uri_str(&uri)
        .await
        .expect("failed to create MongoDB client");
    let config = Config {
        mongodb_uri: uri.clone(),
        default_database: Some("test_db".into()),
        default_collection: Some("test_coll".into()),
        pool_min_size: None,
        pool_max_size: None,
        connect_timeout: None,
        server_selection_timeout: None,
        log_level: None,
        bind_address: "127.0.0.1:3000".into(),
    };
    AppState::new(client, &config)
}

pub fn unique_database() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("test_db_{}", timestamp)
}

pub fn unique_collection() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_coll_{}", timestamp)
}

pub async fn cleanup_test_databases() {
    let uri = mongodb_test_uri();
    match Client::with_uri_str(&uri).await {
        Ok(client) => {
            match client.list_database_names(None, None).await {
                Ok(databases) => {
                    let test_databases: Vec<String> = databases
                        .into_iter()
                        .filter(|db| {
                            // Clean up databases matching test pattern
                            db.starts_with("test_db_")
                        })
                        .collect();

                    if test_databases.is_empty() {
                        eprintln!("No test databases found to clean up.");
                        return;
                    }

                    eprintln!(
                        "\n🧹 Cleaning up {} test database(s)...",
                        test_databases.len()
                    );
                    let mut cleaned = 0;
                    let mut failed = 0;

                    for db_name in &test_databases {
                        match client.database(db_name).drop(None).await {
                            Ok(_) => {
                                eprintln!("  ✓ Dropped: {}", db_name);
                                cleaned += 1;
                            }
                            Err(e) => {
                                eprintln!("  ✗ Failed to drop {}: {}", db_name, e);
                                failed += 1;
                            }
                        }
                    }

                    eprintln!(
                        "✅ Cleanup complete: {} dropped, {} failed\n",
                        cleaned, failed
                    );
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to list databases for cleanup: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "⚠️  Warning: Failed to connect to MongoDB for cleanup: {}",
                e
            );
        }
    }
}
