// src-tauri/src/state.rs
//
// Application state with SQLite database connection
// Extended to support multi-story characters and master reference images

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::sync::Mutex;
use std::fs;
use tauri::{AppHandle, Manager};

pub struct OllamaState {
    pub db: SqlitePool,
    pub client: reqwest::Client,
    pub base_url: String,
    pub current_chat_id: Mutex<Option<i64>>,
}

impl OllamaState {
    pub async fn new(app_handle: &AppHandle) -> Self {
        // Tauri 2.0 uses app_handle.path() instead of tauri::api::path
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Could not determine app data directory");

        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).expect("Failed to create app data directory");
        }

        let db_path = app_dir.join("storyteller.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Failed to connect to SQLite");

        Self::setup_database(&pool).await;

        Self {
            db: pool,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .unwrap(),
            base_url: "http://localhost:11434".to_string(),
            current_chat_id: Mutex::new(None),
        }
    }

    async fn setup_database(pool: &SqlitePool) {
        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(pool)
            .await
            .ok();

        // Chats table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL DEFAULT 'New Chat',
                character_id INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create chats table");

        // Messages table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(chat_id) REFERENCES chats(id) ON DELETE CASCADE
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create messages table");

        // Images table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                chat_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                prompt TEXT,
                FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE,
                FOREIGN KEY(chat_id) REFERENCES chats(id) ON DELETE CASCADE
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create images table");

        // Story premises table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS story_premises (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create story_premises table");

        // =====================================================================
        // CHARACTERS TABLE - Extended for multi-story support
        // =====================================================================
        // Note: We add new columns if they don't exist (migration-safe)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS characters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                story_id INTEGER,
                name TEXT NOT NULL,
                age INTEGER,
                gender TEXT,
                skin_tone TEXT,
                hair_style TEXT,
                hair_color TEXT,
                body_type TEXT,
                personality TEXT,
                additional_notes TEXT,
                default_clothing TEXT,
                sd_prompt TEXT,
                image TEXT,
                master_image_path TEXT,
                seed INTEGER,
                art_style TEXT DEFAULT 'Realistic',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(story_id) REFERENCES story_premises(id) ON DELETE CASCADE
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create characters table");

        // Migration: Add new columns if they don't exist (for existing databases)
        // These will silently fail if columns already exist
        sqlx::query("ALTER TABLE characters ADD COLUMN story_id INTEGER REFERENCES story_premises(id) ON DELETE CASCADE")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE characters ADD COLUMN default_clothing TEXT")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE characters ADD COLUMN master_image_path TEXT")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE characters ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE characters ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP")
            .execute(pool).await.ok();

        // =====================================================================
        // INDEXES for fast lookups
        // =====================================================================
        
        // Index for fast exact name lookups (critical for LLM integration)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_characters_name ON characters(name)")
            .execute(pool).await.ok();

        // Index for story-based queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_characters_story ON characters(story_id)")
            .execute(pool).await.ok();

        // Composite index for name + story lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_characters_story_name ON characters(story_id, name)")
            .execute(pool).await.ok();

        // Index for chat lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_chat ON messages(chat_id)")
            .execute(pool).await.ok();

        // Index for image lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_chat ON images(chat_id)")
            .execute(pool).await.ok();
    }
}