// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri::WebviewWindowBuilder;
use tauri::WebviewUrl;
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicU32;

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
            server_url: "https://po.megacaresa.com/".to_string(),
            window_width: 1400,
            window_height: 900,
            fullscreen: false,
        }
    }
}

struct AppState {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
}

// Counter for unique popup window labels
static POPUP_COUNTER: AtomicU32 = AtomicU32::new(0);

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

#[tauri::command]
fn print_page(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.print().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[tauri::command]
fn open_in_browser(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_popup_window(
    app: tauri::AppHandle,
    url: String,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
    let popup_id = POPUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let label = format!("popup-{}", popup_id);

    let window_title = title.unwrap_or_else(|| "VOPECS POS".to_string());
    let window_width = width.unwrap_or(800.0);
    let window_height = height.unwrap_or(600.0);

    WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
    )
    .title(&window_title)
    .inner_size(window_width, window_height)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to create popup: {}", e))?;

    Ok(label)
}

#[tauri::command]
fn open_print_window(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let popup_id = POPUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let label = format!("print-{}", popup_id);
    let label_for_html = label.clone();

    // Create HTML with proper Arabic support and auto-print script
    let print_html = format!(
        r#"<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>طباعة</title>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Cairo:wght@400;600;700&display=swap');
        * {{ font-family: 'Cairo', 'Segoe UI', Tahoma, Arial, sans-serif !important; margin: 0; padding: 0; box-sizing: border-box; }}
        html, body {{ direction: rtl; text-align: right; margin: 0; padding: 0; }}
        @media print {{
            html, body {{ margin: 0 !important; padding: 0 !important; }}
            @page {{ margin: 5mm; }}
            .no-print {{ display: none !important; }}
        }}
        .print-actions {{
            position: fixed;
            top: 10px;
            left: 10px;
            z-index: 9999;
            display: flex;
            gap: 10px;
        }}
        .print-actions button {{
            padding: 8px 16px;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-family: inherit;
            font-size: 14px;
        }}
        .btn-print {{
            background: #22c55e;
            color: white;
        }}
        .btn-close {{
            background: #ef4444;
            color: white;
        }}
    </style>
</head>
<body>
    <div class="print-actions no-print">
        <button class="btn-print" onclick="doPrint()">طباعة</button>
        <button class="btn-close" onclick="closeWindow()">إغلاق</button>
    </div>
    {}
    <script>
        var printAttempted = false;
        var windowLabel = '{}';

        function doPrint() {{
            if (printAttempted) return;
            printAttempted = true;
            window.print();
            setTimeout(function() {{ printAttempted = false; }}, 1000);
        }}

        function closeWindow() {{
            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {{
                window.__TAURI_INTERNALS__.invoke('close_popup_window', {{ label: windowLabel }});
            }} else {{
                window.close();
            }}
        }}

        window.onload = function() {{
            setTimeout(doPrint, 800);
        }};

        window.onafterprint = function() {{
            setTimeout(closeWindow, 500);
        }};
    </script>
</body>
</html>"#,
        content, label_for_html
    );

    // Use data URL - works on all platforms including Windows WebView2
    let encoded_html = url_encode_html(&print_html);
    let data_url = format!("data:text/html;charset=utf-8,{}", encoded_html);

    let _window = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(data_url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
    )
    .title("طباعة")
    .inner_size(450.0, 500.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to create print window: {}", e))?;

    Ok(())
}

// URL encode HTML for data URL
fn url_encode_html(html: &str) -> String {
    let mut encoded = String::with_capacity(html.len() * 3);
    for byte in html.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", byte));
            }
        }
    }
    encoded
}

fn base64_encode(input: &str) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder = Base64Encoder::new(&mut buf);
        encoder.write_all(input.as_bytes()).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

struct Base64Encoder<W: std::io::Write> {
    writer: W,
}

impl<W: std::io::Write> Base64Encoder<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: std::io::Write> std::io::Write for Base64Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        for chunk in buf.chunks(3) {
            let b0 = chunk[0] as usize;
            let b1 = chunk.get(1).map(|&b| b as usize).unwrap_or(0);
            let b2 = chunk.get(2).map(|&b| b as usize).unwrap_or(0);

            let c0 = ALPHABET[b0 >> 2];
            let c1 = ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)];
            let c2 = if chunk.len() > 1 { ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] } else { b'=' };
            let c3 = if chunk.len() > 2 { ALPHABET[b2 & 0x3f] } else { b'=' };

            self.writer.write_all(&[c0, c1, c2, c3])?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[tauri::command]
fn close_popup_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// JavaScript to inject into the webview for handling popups and print
const POPUP_HANDLER_SCRIPT: &str = r#"
(function() {
    // Auto-reload if page failed to load (white screen fix)
    if (!window.__VOPECS_RELOAD_CHECK__) {
        window.__VOPECS_RELOAD_CHECK__ = true;
        window.__VOPECS_RELOAD_ATTEMPTS__ = window.__VOPECS_RELOAD_ATTEMPTS__ || 0;

        function checkAndReload() {
            var body = document.body;
            var html = document.documentElement;
            var isBlank = !body || !html || body.innerHTML.trim() === '' || body.children.length === 0;
            var hasErrorText = body && (
                body.innerText.includes('This site can') ||
                body.innerText.includes('ERR_') ||
                body.innerText.includes('refused to connect')
            );
            var hasAppContent = document.querySelector('#app') ||
                               document.querySelector('.app-content') ||
                               document.querySelector('[data-v-]') ||
                               (body && body.children.length > 3);

            if ((isBlank || hasErrorText) && !hasAppContent && window.__VOPECS_RELOAD_ATTEMPTS__ < 5) {
                window.__VOPECS_RELOAD_ATTEMPTS__++;
                setTimeout(function() { location.reload(); }, 500);
            }
        }
        setTimeout(checkAndReload, 2000);
    }

    // Initialize popup and print handlers
    function initPopupHandlers() {
        if (window.__VOPECS_INITIALIZED__) return;
        window.__VOPECS_INITIALIZED__ = true;
        console.log('[VOPECS] Initializing handlers...');

        // IFRAME-BASED PRINT - Works reliably on all platforms including Windows
        function printWithIframe(htmlContent) {
            console.log('[VOPECS] Printing with iframe, content length:', htmlContent.length);

            // Remove any existing print iframe
            var existingFrame = document.getElementById('vopecs-print-frame');
            if (existingFrame) existingFrame.remove();

            // Create hidden iframe
            var iframe = document.createElement('iframe');
            iframe.id = 'vopecs-print-frame';
            iframe.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;';
            document.body.appendChild(iframe);

            var iframeDoc = iframe.contentWindow.document;
            iframeDoc.open();
            iframeDoc.write(htmlContent);
            iframeDoc.close();

            // Wait for content to load then print
            setTimeout(function() {
                try {
                    iframe.contentWindow.focus();
                    iframe.contentWindow.print();
                } catch(e) {
                    console.error('[VOPECS] Print error:', e);
                }
                // Clean up after print dialog closes
                setTimeout(function() {
                    iframe.remove();
                }, 1000);
            }, 500);
        }

        // Create mock window that uses iframe for printing
        function createPrintWindow() {
            var content = '';

            var mockDoc = {
                write: function(html) { content += html; },
                writeln: function(html) { content += html + '\n'; },
                close: function() {
                    console.log('[VOPECS] Document closed, printing...');
                    setTimeout(function() { printWithIframe(content); }, 100);
                },
                open: function() { content = ''; return mockDoc; },
                createElement: function(tag) {
                    return document.createElement(tag);
                },
                createTextNode: function(text) {
                    return document.createTextNode(text);
                },
                body: { appendChild: function(){}, innerHTML: '' },
                head: { appendChild: function(){} },
                documentElement: { appendChild: function(){} }
            };

            return {
                document: mockDoc,
                print: function() { printWithIframe(content); },
                close: function() { console.log('[VOPECS] Window closed'); },
                focus: function() {},
                blur: function() {},
                closed: false,
                location: { href: 'about:blank' },
                opener: window
            };
        }

        // Store original window.open
        window.__VOPECS_ORIGINAL_OPEN__ = window.open;

        // Override window.open
        window.open = function(url, target, features) {
            console.log('[VOPECS] window.open:', url, target);

            // Empty URL = print popup, use iframe method
            if (!url || url === '' || url === 'about:blank') {
                return createPrintWindow();
            }

            // javascript: URLs - ignore
            if (url && url.startsWith('javascript:')) {
                return null;
            }

            // Convert relative URLs to absolute
            var absoluteUrl = url;
            try {
                if (url && !url.match(/^(https?|file|blob|data):/)) {
                    absoluteUrl = new URL(url, window.location.href).href;
                }
            } catch(e) {}

            // Parse width/height from features
            if (features) {
                var wm = features.match(/width=(\d+)/);
                var hm = features.match(/height=(\d+)/);
                if (wm) width = parseInt(wm[1]);
                if (hm) height = parseInt(hm[1]);
            }

            // For actual URLs, open in system browser (most reliable on Windows)
            console.log('[VOPECS] Opening URL in browser:', absoluteUrl);
            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                window.__TAURI_INTERNALS__.invoke('open_in_browser', { url: absoluteUrl });
            }

            // Return mock window for compatibility
            return {
                document: { write: function(){}, close: function(){}, body: {} },
                close: function() {},
                focus: function() {},
                closed: false,
                location: { href: absoluteUrl }
            };
        };

        // Handle target="_blank" links - open in browser
        document.addEventListener('click', function(e) {
            var el = e.target;
            while (el && el.tagName !== 'A') el = el.parentElement;
            if (el && el.tagName === 'A' && el.target === '_blank' && el.href) {
                e.preventDefault();
                console.log('[VOPECS] Opening link in browser:', el.href);
                if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                    window.__TAURI_INTERNALS__.invoke('open_in_browser', { url: el.href });
                }
            }
        }, true);

        console.log('[VOPECS] Handlers initialized');
    }

    initPopupHandlers();
})();
"#;

// Check for updates from GitHub releases
async fn check_for_updates(app: tauri::AppHandle) {
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    let version = update.version.clone();
                    let msg = format!("A new version ({}) is available. Do you want to download and install it?", version);

                    let confirmed = app.dialog()
                        .message(msg)
                        .title("Update Available")
                        .buttons(tauri_plugin_dialog::MessageDialogButtons::OkCancel)
                        .kind(MessageDialogKind::Info)
                        .blocking_show();

                    if confirmed {
                        // Show downloading message
                        let app_clone = app.clone();

                        // Download and install
                        match update.download_and_install(
                            |downloaded, total| {
                                if let Some(t) = total {
                                    let percent = (downloaded as f64 / t as f64 * 100.0) as u32;
                                    println!("Downloading update: {}%", percent);
                                }
                            },
                            || {
                                println!("Download complete, installing...");
                            }
                        ).await {
                            Ok(_) => {
                                app_clone.dialog()
                                    .message("Update installed successfully. The app will now restart.")
                                    .title("Update Complete")
                                    .kind(MessageDialogKind::Info)
                                    .blocking_show();

                                // Restart the app
                                app_clone.restart();
                            }
                            Err(e) => {
                                app_clone.dialog()
                                    .message(format!("Failed to install update: {}", e))
                                    .title("Update Error")
                                    .kind(MessageDialogKind::Error)
                                    .blocking_show();
                            }
                        }
                    }
                }
                Ok(None) => {
                    app.dialog()
                        .message("You are running the latest version.")
                        .title("No Updates")
                        .kind(MessageDialogKind::Info)
                        .blocking_show();
                }
                Err(e) => {
                    app.dialog()
                        .message(format!("Failed to check for updates: {}", e))
                        .title("Update Error")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                }
            }
        }
        Err(e) => {
            eprintln!("Updater not available: {}", e);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|_window, event| {
            // Handle window events if needed
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    // Allow closing popup windows
                }
                _ => {}
            }
        })
        .on_page_load(|webview, _payload| {
            // Inject popup handler script when page loads
            let _ = webview.eval(POPUP_HANDLER_SCRIPT);
        })
        .setup(|app| {
            let settings_path = get_settings_path(app);
            let settings = load_settings(&settings_path);

            // Store state
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
            });

            // Navigate to saved URL
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(url) = settings.server_url.parse() {
                    let _ = window.navigate(url);
                }
            }

            // Create menu (English labels)
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, Some("CmdOrCtrl+,"))?;
            let reload_item = MenuItem::with_id(app, "reload", "Reload", true, Some("CmdOrCtrl+R"))?;
            let clear_cache_item = MenuItem::with_id(app, "clear_cache", "Clear Cache", true, Some("CmdOrCtrl+Shift+R"))?;
            let fullscreen_item = MenuItem::with_id(app, "fullscreen", "Fullscreen", true, Some("F11"))?;
            let check_update_item = MenuItem::with_id(app, "check_update", "Check for Updates", true, None::<&str>)?;
            let devtools_item = MenuItem::with_id(app, "devtools", "Developer Tools", true, Some("CmdOrCtrl+Shift+I"))?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, Some("CmdOrCtrl+Q"))?;

            let app_menu = Submenu::with_items(
                app,
                "VOPECS POS",
                true,
                &[&settings_item, &reload_item, &clear_cache_item, &fullscreen_item, &check_update_item, &devtools_item, &quit_item],
            )?;

            let menu = Menu::with_items(app, &[&app_menu])?;
            app.set_menu(menu)?;

            // Auto-reload mechanism for white screen fix
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Wait for initial load attempt
                std::thread::sleep(std::time::Duration::from_secs(3));

                for attempt in 1..=5 {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        // Force reload if page is blank
                        let _ = window.eval(&format!(
                            "if (!document.body || document.body.children.length < 3) {{ console.log('[VOPECS] Attempt {} - reloading...'); location.reload(); }}",
                            attempt
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            });

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
                    "clear_cache" => {
                        if let Some(window) = app.get_webview_window("main") {
                            // Clear WebView cache and reload
                            let _ = window.eval("
                                if ('caches' in window) {
                                    caches.keys().then(names => {
                                        names.forEach(name => caches.delete(name));
                                    });
                                }
                                if ('serviceWorker' in navigator) {
                                    navigator.serviceWorker.getRegistrations().then(regs => {
                                        regs.forEach(reg => reg.unregister());
                                    });
                                }
                                localStorage.clear();
                                sessionStorage.clear();
                                location.reload(true);
                            ");
                        }
                    }
                    "fullscreen" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(is_fullscreen) = window.is_fullscreen() {
                                let _ = window.set_fullscreen(!is_fullscreen);
                            }
                        }
                    }
                    "check_update" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            check_for_updates(app_handle).await;
                        });
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
            open_popup_window,
            close_popup_window,
            print_page,
            open_in_browser,
            open_print_window,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running VOPECS POS");
}
