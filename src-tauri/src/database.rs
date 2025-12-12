use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::fs;
use tauri::Manager;

/// Get the database path in app data directory
pub fn get_db_path(app: &tauri::App) -> PathBuf {
    let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
    fs::create_dir_all(&app_data_dir).ok();
    app_data_dir.join("vopecs_pos.db")
}

/// Initialize the database with required tables
pub fn init_database(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create products table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            price REAL NOT NULL DEFAULT 0,
            cost REAL,
            category_id INTEGER,
            brand_id INTEGER,
            unit_id INTEGER,
            sale_unit_id INTEGER,
            tax_method TEXT,
            tax_percent REAL DEFAULT 0,
            discount REAL DEFAULT 0,
            discount_method TEXT,
            image TEXT,
            is_service INTEGER DEFAULT 0,
            stock_qty REAL DEFAULT 0,
            min_stock REAL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create index on product code for fast lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_products_code ON products(code)",
        [],
    )?;

    // Create index on product name for search
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_products_name ON products(name)",
        [],
    )?;

    // Create clients table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            phone TEXT,
            email TEXT,
            address TEXT,
            tax_number TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create categories table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id INTEGER,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create warehouses table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS warehouses (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create payment_methods table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS payment_methods (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create offline_sales table for storing sales made offline
    conn.execute(
        "CREATE TABLE IF NOT EXISTS offline_sales (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            local_ref TEXT NOT NULL UNIQUE,
            client_id INTEGER,
            warehouse_id INTEGER NOT NULL,
            grand_total REAL NOT NULL,
            paid_amount REAL NOT NULL,
            tax_amount REAL DEFAULT 0,
            discount REAL DEFAULT 0,
            payment_method_id INTEGER NOT NULL,
            details_json TEXT NOT NULL,
            payments_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            synced_at TEXT,
            server_sale_id INTEGER,
            error_message TEXT
        )",
        [],
    )?;

    // Create index on offline_sales status for sync queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_offline_sales_status ON offline_sales(status)",
        [],
    )?;

    // Create sync_log table to track sync operations
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            operation TEXT NOT NULL,
            record_count INTEGER DEFAULT 0,
            status TEXT NOT NULL,
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create settings table for app settings
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    println!("Database schema initialized successfully");
    Ok(())
}
