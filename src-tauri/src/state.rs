use sqlx::sqlite::SqlitePool;
use std::sync::Mutex;
use tauri::api::path::app_data_dir;
use std::fs;

pub struct AppState {
    pub db: SqlitePool,
    pub client: reqwest::Client,
    pub base_url: String,
    pub current_chat_id: Mutex<Option<i64>>,
}

impl AppState {
    pub async fn new(app_handle: &tauri::AppHandle) -> Self {
        let app_dir = app_data_dir(&app_handle.config())
            .expect("Could not determine app data directory");
        
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).expect("Failed to create app data directory");
        }

        let db_path = app_dir.join("storyteller.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to SQLite");

        Self::setup_database(&pool).await;

        Self {
            db: pool,
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
            current_chat_id: Mutex::new(None),
        }
    }

    async fn setup_database(pool: &SqlitePool) {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS story_premises (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).execute(pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(chat_id) REFERENCES chats(id)
            )"
        ).execute(pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                chat_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                prompt TEXT,
                FOREIGN KEY(message_id) REFERENCES messages(id),
                FOREIGN KEY(chat_id) REFERENCES chats(id)
            )"
        ).execute(pool).await.unwrap();
    }
}