// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri::WebviewWindowBuilder;
use tauri::WebviewUrl;
use tauri::menu::{Menu, MenuItem, Submenu};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::{Connection, params};

mod database;
use database::{init_database, get_db_path};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub server_url: String,
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            server_url: "http://vopecspos.test/".to_string(),
            window_width: 1400,
            window_height: 900,
            fullscreen: false,
        }
    }
}

struct AppState {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    db_path: PathBuf,
}

fn get_settings_path(app: &tauri::App) -> PathBuf {
    let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
    fs::create_dir_all(&app_data_dir).ok();
    app_data_dir.join("settings.json")
}

fn load_settings(path: &PathBuf) -> AppSettings {
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    AppSettings::default()
}

fn save_settings_to_file(path: &PathBuf, settings: &AppSettings) -> Result<(), String> {
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(path, content)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
fn save_settings(
    state: tauri::State<AppState>,
    new_settings: AppSettings,
) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    *settings = new_settings.clone();
    save_settings_to_file(&state.settings_path, &new_settings)?;
    Ok(())
}

#[tauri::command]
fn get_server_url(state: tauri::State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.server_url.clone())
}

#[tauri::command]
fn set_server_url(state: tauri::State<AppState>, url: String) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.server_url = url;
    save_settings_to_file(&state.settings_path, &settings)?;
    Ok(())
}

#[tauri::command]
fn toggle_fullscreen(window: tauri::Window) -> Result<(), String> {
    let is_fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    window.set_fullscreen(!is_fullscreen).map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== DATABASE COMMANDS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub price: f64,
    pub cost: Option<f64>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub sale_unit_id: Option<i64>,
    pub tax_method: Option<String>,
    pub tax_percent: Option<f64>,
    pub discount: Option<f64>,
    pub discount_method: Option<String>,
    pub image: Option<String>,
    pub is_service: bool,
    pub stock_qty: f64,
    pub min_stock: Option<f64>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OfflineSale {
    pub id: Option<i64>,
    pub local_ref: String,
    pub client_id: Option<i64>,
    pub warehouse_id: i64,
    pub grand_total: f64,
    pub paid_amount: f64,
    pub tax_amount: f64,
    pub discount: f64,
    pub payment_method_id: i64,
    pub details_json: String,
    pub payments_json: String,
    pub status: String,
    pub created_at: String,
    pub synced_at: Option<String>,
    pub server_sale_id: Option<i64>,
    pub error_message: Option<String>,
}

#[tauri::command]
fn db_get_products(state: tauri::State<AppState>) -> Result<Vec<Product>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, code, name, price, cost, category_id, brand_id, unit_id, sale_unit_id,
                tax_method, tax_percent, discount, discount_method, image, is_service,
                stock_qty, min_stock, updated_at
         FROM products ORDER BY name"
    ).map_err(|e| e.to_string())?;

    let products = stmt.query_map([], |row| {
        Ok(Product {
            id: row.get(0)?,
            code: row.get(1)?,
            name: row.get(2)?,
            price: row.get(3)?,
            cost: row.get(4)?,
            category_id: row.get(5)?,
            brand_id: row.get(6)?,
            unit_id: row.get(7)?,
            sale_unit_id: row.get(8)?,
            tax_method: row.get(9)?,
            tax_percent: row.get(10)?,
            discount: row.get(11)?,
            discount_method: row.get(12)?,
            image: row.get(13)?,
            is_service: row.get(14)?,
            stock_qty: row.get(15)?,
            min_stock: row.get(16)?,
            updated_at: row.get(17)?,
        })
    }).map_err(|e| e.to_string())?;

    let result: Vec<Product> = products.filter_map(|p| p.ok()).collect();
    Ok(result)
}

#[tauri::command]
fn db_get_product_by_code(state: tauri::State<AppState>, code: String) -> Result<Option<Product>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, code, name, price, cost, category_id, brand_id, unit_id, sale_unit_id,
                tax_method, tax_percent, discount, discount_method, image, is_service,
                stock_qty, min_stock, updated_at
         FROM products WHERE code = ? LIMIT 1"
    ).map_err(|e| e.to_string())?;

    let product = stmt.query_row([&code], |row| {
        Ok(Product {
            id: row.get(0)?,
            code: row.get(1)?,
            name: row.get(2)?,
            price: row.get(3)?,
            cost: row.get(4)?,
            category_id: row.get(5)?,
            brand_id: row.get(6)?,
            unit_id: row.get(7)?,
            sale_unit_id: row.get(8)?,
            tax_method: row.get(9)?,
            tax_percent: row.get(10)?,
            discount: row.get(11)?,
            discount_method: row.get(12)?,
            image: row.get(13)?,
            is_service: row.get(14)?,
            stock_qty: row.get(15)?,
            min_stock: row.get(16)?,
            updated_at: row.get(17)?,
        })
    }).ok();

    Ok(product)
}

#[tauri::command]
fn db_search_products(state: tauri::State<AppState>, query: String) -> Result<Vec<Product>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let search_pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT id, code, name, price, cost, category_id, brand_id, unit_id, sale_unit_id,
                tax_method, tax_percent, discount, discount_method, image, is_service,
                stock_qty, min_stock, updated_at
         FROM products
         WHERE name LIKE ? OR code LIKE ?
         ORDER BY name LIMIT 50"
    ).map_err(|e| e.to_string())?;

    let products = stmt.query_map([&search_pattern, &search_pattern], |row| {
        Ok(Product {
            id: row.get(0)?,
            code: row.get(1)?,
            name: row.get(2)?,
            price: row.get(3)?,
            cost: row.get(4)?,
            category_id: row.get(5)?,
            brand_id: row.get(6)?,
            unit_id: row.get(7)?,
            sale_unit_id: row.get(8)?,
            tax_method: row.get(9)?,
            tax_percent: row.get(10)?,
            discount: row.get(11)?,
            discount_method: row.get(12)?,
            image: row.get(13)?,
            is_service: row.get(14)?,
            stock_qty: row.get(15)?,
            min_stock: row.get(16)?,
            updated_at: row.get(17)?,
        })
    }).map_err(|e| e.to_string())?;

    let result: Vec<Product> = products.filter_map(|p| p.ok()).collect();
    Ok(result)
}

#[tauri::command]
fn db_save_products(state: tauri::State<AppState>, products: Vec<Product>) -> Result<i64, String> {
    let mut conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Clear existing products
    tx.execute("DELETE FROM products", []).map_err(|e| e.to_string())?;

    for product in &products {
        tx.execute(
            "INSERT INTO products (id, code, name, price, cost, category_id, brand_id, unit_id,
                                   sale_unit_id, tax_method, tax_percent, discount, discount_method,
                                   image, is_service, stock_qty, min_stock, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                product.id, product.code, product.name, product.price, product.cost,
                product.category_id, product.brand_id, product.unit_id, product.sale_unit_id,
                product.tax_method, product.tax_percent, product.discount, product.discount_method,
                product.image, product.is_service, product.stock_qty, product.min_stock, product.updated_at
            ],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(products.len() as i64)
}

#[tauri::command]
fn db_get_clients(state: tauri::State<AppState>) -> Result<Vec<Client>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, name, phone, email, address, tax_number, updated_at FROM clients ORDER BY name"
    ).map_err(|e| e.to_string())?;

    let clients = stmt.query_map([], |row| {
        Ok(Client {
            id: row.get(0)?,
            name: row.get(1)?,
            phone: row.get(2)?,
            email: row.get(3)?,
            address: row.get(4)?,
            tax_number: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let result: Vec<Client> = clients.filter_map(|c| c.ok()).collect();
    Ok(result)
}

#[tauri::command]
fn db_save_clients(state: tauri::State<AppState>, clients: Vec<Client>) -> Result<i64, String> {
    let mut conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Clear existing clients
    tx.execute("DELETE FROM clients", []).map_err(|e| e.to_string())?;

    for client in &clients {
        tx.execute(
            "INSERT INTO clients (id, name, phone, email, address, tax_number, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                client.id, client.name, client.phone, client.email,
                client.address, client.tax_number, client.updated_at
            ],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(clients.len() as i64)
}

#[tauri::command]
fn db_save_offline_sale(state: tauri::State<AppState>, sale: OfflineSale) -> Result<i64, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO offline_sales (local_ref, client_id, warehouse_id, grand_total, paid_amount,
                                    tax_amount, discount, payment_method_id, details_json,
                                    payments_json, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            sale.local_ref, sale.client_id, sale.warehouse_id, sale.grand_total,
            sale.paid_amount, sale.tax_amount, sale.discount, sale.payment_method_id,
            sale.details_json, sale.payments_json, "pending", sale.created_at
        ],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
fn db_get_pending_sales(state: tauri::State<AppState>) -> Result<Vec<OfflineSale>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, local_ref, client_id, warehouse_id, grand_total, paid_amount, tax_amount,
                discount, payment_method_id, details_json, payments_json, status, created_at,
                synced_at, server_sale_id, error_message
         FROM offline_sales WHERE status = 'pending' ORDER BY created_at"
    ).map_err(|e| e.to_string())?;

    let sales = stmt.query_map([], |row| {
        Ok(OfflineSale {
            id: row.get(0)?,
            local_ref: row.get(1)?,
            client_id: row.get(2)?,
            warehouse_id: row.get(3)?,
            grand_total: row.get(4)?,
            paid_amount: row.get(5)?,
            tax_amount: row.get(6)?,
            discount: row.get(7)?,
            payment_method_id: row.get(8)?,
            details_json: row.get(9)?,
            payments_json: row.get(10)?,
            status: row.get(11)?,
            created_at: row.get(12)?,
            synced_at: row.get(13)?,
            server_sale_id: row.get(14)?,
            error_message: row.get(15)?,
        })
    }).map_err(|e| e.to_string())?;

    let result: Vec<OfflineSale> = sales.filter_map(|s| s.ok()).collect();
    Ok(result)
}

#[tauri::command]
fn db_mark_sale_synced(state: tauri::State<AppState>, local_id: i64, server_sale_id: i64) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE offline_sales SET status = 'synced', server_sale_id = ?1, synced_at = datetime('now') WHERE id = ?2",
        params![server_sale_id, local_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn db_mark_sale_failed(state: tauri::State<AppState>, local_id: i64, error: String) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE offline_sales SET status = 'failed', error_message = ?1 WHERE id = ?2",
        params![error, local_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn db_get_products_count(state: tauri::State<AppState>) -> Result<i64, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
fn db_get_pending_sales_count(state: tauri::State<AppState>) -> Result<i64, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM offline_sales WHERE status = 'pending'", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
fn db_update_product_stock(state: tauri::State<AppState>, product_id: i64, new_qty: f64) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE products SET stock_qty = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_qty, product_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

fn open_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    // Check if already open
    if app.get_webview_window("settings").is_some() {
        return Ok(());
    }

    // Use tauri:// protocol to load from dist folder (bundled with app)
    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("settings.html".into())
    )
    .title("إعدادات التطبيق")
    .inner_size(500.0, 550.0)
    .resizable(false)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    open_settings_window(&app)
}

#[tauri::command]
fn open_main_devtools(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|app| {
            let settings_path = get_settings_path(app);
            let settings = load_settings(&settings_path);

            // Initialize SQLite database
            let db_path = get_db_path(app);
            if let Err(e) = init_database(&db_path) {
                eprintln!("Failed to initialize database: {}", e);
            } else {
                println!("Database initialized at: {:?}", db_path);
            }

            // Store state
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                db_path,
            });

            // Create menu
            let settings_item = MenuItem::with_id(app, "settings", "⚙️ الإعدادات", true, Some("CmdOrCtrl+,"))?;
            let reload_item = MenuItem::with_id(app, "reload", "🔄 إعادة تحميل", true, Some("CmdOrCtrl+R"))?;
            let fullscreen_item = MenuItem::with_id(app, "fullscreen", "📺 ملء الشاشة", true, Some("F11"))?;
            let devtools_item = MenuItem::with_id(app, "devtools", "🔧 Developer Tools", true, Some("CmdOrCtrl+Shift+I"))?;
            let quit_item = MenuItem::with_id(app, "quit", "❌ خروج", true, Some("CmdOrCtrl+Q"))?;

            let app_menu = Submenu::with_items(
                app,
                "VOPECS POS",
                true,
                &[&settings_item, &reload_item, &fullscreen_item, &devtools_item, &quit_item],
            )?;

            let menu = Menu::with_items(app, &[&app_menu])?;
            app.set_menu(menu)?;

            // Handle menu events
            app.on_menu_event(move |app, event| {
                match event.id().as_ref() {
                    "settings" => {
                        let _ = open_settings_window(app);
                    }
                    "reload" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let state: tauri::State<AppState> = app.state();
                            let url = {
                                let settings = state.settings.lock().unwrap();
                                settings.server_url.clone()
                            };
                            let _ = window.navigate(url.parse().unwrap());
                        }
                    }
                    "fullscreen" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(is_fullscreen) = window.is_fullscreen() {
                                let _ = window.set_fullscreen(!is_fullscreen);
                            }
                        }
                    }
                    "devtools" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.open_devtools();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_server_url,
            set_server_url,
            toggle_fullscreen,
            open_settings,
            open_main_devtools,
            // Database commands
            db_get_products,
            db_get_product_by_code,
            db_search_products,
            db_save_products,
            db_get_clients,
            db_save_clients,
            db_save_offline_sale,
            db_get_pending_sales,
            db_mark_sale_synced,
            db_mark_sale_failed,
            db_get_products_count,
            db_get_pending_sales_count,
            db_update_product_stock,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running VOPECS POS");
}
