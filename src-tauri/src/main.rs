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

    // Create HTML with proper Arabic support - minimal margins for single page print
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
        }}
    </style>
</head>
<body>{}</body>
</html>"#,
        content
    );

    // Save to temp file for proper origin (file:// URLs work better with print)
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("vopecs_print_{}.html", popup_id));
    fs::write(&temp_file, &print_html)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    let file_url = format!("file://{}", temp_file.to_string_lossy());

    let window = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(file_url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
    )
    .title("طباعة")
    .inner_size(450.0, 400.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to create print window: {}", e))?;

    // Auto-trigger print dialog from Rust after window loads
    let window_clone = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let _ = window_clone.print();
    });

    // Clean up temp file after a delay
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let _ = fs::remove_file(&temp_file);
    });

    Ok(())
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

            // Check various signs of a failed/blank page
            var isBlank = !body || !html || body.innerHTML.trim() === '' || body.children.length === 0;
            var hasErrorText = body && (
                body.innerText.includes('This site can') ||
                body.innerText.includes('ERR_') ||
                body.innerText.includes('refused to connect') ||
                body.innerText.includes('not be reached') ||
                body.innerText.includes('took too long')
            );
            var hasErrorTitle = document.title.includes('Error') || document.title === '';

            // Check if Vue/Angular app has mounted (look for app content)
            var hasAppContent = document.querySelector('#app') ||
                               document.querySelector('.app-content') ||
                               document.querySelector('[data-v-]') ||
                               document.querySelector('main') ||
                               (body && body.children.length > 3);

            var needsReload = (isBlank || hasErrorText) && !hasAppContent;

            if (needsReload && window.__VOPECS_RELOAD_ATTEMPTS__ < 10) {
                window.__VOPECS_RELOAD_ATTEMPTS__++;
                console.log('[VOPECS Desktop] Page appears blank or has error, reloading... (attempt ' + window.__VOPECS_RELOAD_ATTEMPTS__ + ')');
                setTimeout(function() { location.reload(); }, 500);
            } else if (!hasAppContent && window.__VOPECS_RELOAD_ATTEMPTS__ < 10) {
                // Schedule another check
                setTimeout(checkAndReload, 1000);
            }
        }

        // Check at multiple intervals to catch different loading states
        setTimeout(checkAndReload, 1000);
        setTimeout(checkAndReload, 3000);
        setTimeout(checkAndReload, 5000);
    }

    // Wait for Tauri to be ready
    function initPopupHandlers() {
        console.log('[VOPECS Desktop] Initializing popup handlers...');
        console.log('[VOPECS Desktop] __TAURI__ available:', typeof window.__TAURI__ !== 'undefined');
        console.log('[VOPECS Desktop] __TAURI_INTERNALS__ available:', typeof window.__TAURI_INTERNALS__ !== 'undefined');

        // Store original window.open (only once)
        if (window.__VOPECS_ORIGINAL_OPEN__) return;
        window.__VOPECS_ORIGINAL_OPEN__ = window.open;

        // Create a mock window for print popups
        function createPrintWindow() {
            let content = '';
            let printFrame = null;

            function doPrint() {
                console.log('[VOPECS Desktop] Executing print, content length:', content.length);

                // Create a blob URL from the content
                let blob = new Blob([content], { type: 'text/html' });
                let blobUrl = URL.createObjectURL(blob);
                console.log('[VOPECS Desktop] Created blob URL:', blobUrl);

                // Open a new Tauri window with the blob content
                if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                    console.log('[VOPECS Desktop] Opening print window via Tauri...');
                    window.__TAURI_INTERNALS__.invoke('open_print_window', {
                        content: content
                    }).then(function() {
                        console.log('[VOPECS Desktop] Print window opened');
                        URL.revokeObjectURL(blobUrl);
                    }).catch(function(err) {
                        console.error('[VOPECS Desktop] Failed to open print window:', err);
                        URL.revokeObjectURL(blobUrl);
                    });
                } else {
                    console.error('[VOPECS Desktop] Tauri not available');
                    URL.revokeObjectURL(blobUrl);
                }
            }

            // Create a mock element that supports common DOM operations
            function createMockElement(tagName) {
                var children = [];
                var attributes = {};
                var styles = {};
                var element = {
                    tagName: tagName.toUpperCase(),
                    nodeName: tagName.toUpperCase(),
                    nodeType: 1,
                    children: children,
                    childNodes: children,
                    parentNode: null,
                    innerHTML: '',
                    innerText: '',
                    textContent: '',
                    className: '',
                    id: '',
                    style: new Proxy(styles, {
                        get: function(target, prop) { return target[prop] || ''; },
                        set: function(target, prop, value) { target[prop] = value; return true; }
                    }),
                    setAttribute: function(name, value) { attributes[name] = value; },
                    getAttribute: function(name) { return attributes[name] || null; },
                    removeAttribute: function(name) { delete attributes[name]; },
                    hasAttribute: function(name) { return name in attributes; },
                    appendChild: function(child) {
                        children.push(child);
                        child.parentNode = element;
                        return child;
                    },
                    removeChild: function(child) {
                        var idx = children.indexOf(child);
                        if (idx > -1) children.splice(idx, 1);
                        return child;
                    },
                    insertBefore: function(newNode, refNode) {
                        var idx = children.indexOf(refNode);
                        if (idx > -1) children.splice(idx, 0, newNode);
                        else children.push(newNode);
                        newNode.parentNode = element;
                        return newNode;
                    },
                    cloneNode: function(deep) { return createMockElement(tagName); },
                    addEventListener: function() {},
                    removeEventListener: function() {},
                    dispatchEvent: function() { return true; },
                    querySelector: function() { return null; },
                    querySelectorAll: function() { return []; },
                    getElementsByTagName: function() { return []; },
                    getElementsByClassName: function() { return []; },
                    getElementById: function() { return null; },
                    focus: function() {},
                    blur: function() {},
                    click: function() {}
                };
                return element;
            }

            var mockHead = createMockElement('head');
            var mockBody = createMockElement('body');
            var mockHtml = createMockElement('html');
            mockHtml.appendChild(mockHead);
            mockHtml.appendChild(mockBody);

            let mockDoc = {
                write: function(html) {
                    content += html;
                    console.log('[VOPECS Desktop] Print window write:', html.substring(0, 100) + '...');
                },
                writeln: function(html) {
                    content += html + '\n';
                },
                close: function() {
                    console.log('[VOPECS Desktop] Print window document closed, triggering print...');
                    // Auto-print when document is closed (common pattern)
                    setTimeout(doPrint, 100);
                },
                open: function() {
                    content = '';
                    return mockDoc;
                },
                createElement: function(tagName) {
                    return createMockElement(tagName);
                },
                createTextNode: function(text) {
                    return { nodeType: 3, textContent: text, nodeName: '#text' };
                },
                createDocumentFragment: function() {
                    return createMockElement('fragment');
                },
                body: mockBody,
                head: mockHead,
                documentElement: mockHtml,
                title: '',
                querySelector: function(sel) {
                    if (sel === 'head') return mockHead;
                    if (sel === 'body') return mockBody;
                    if (sel === 'html') return mockHtml;
                    return null;
                },
                querySelectorAll: function(sel) {
                    if (sel === 'head') return [mockHead];
                    if (sel === 'body') return [mockBody];
                    return [];
                },
                getElementById: function(id) { return null; },
                getElementsByTagName: function(tag) {
                    tag = tag.toLowerCase();
                    if (tag === 'head') return [mockHead];
                    if (tag === 'body') return [mockBody];
                    if (tag === 'html') return [mockHtml];
                    return [];
                },
                getElementsByClassName: function(cls) { return []; }
            };

            let mockWindow = {
                document: mockDoc,
                print: function() {
                    console.log('[VOPECS Desktop] Mock window.print() called');
                    doPrint();
                },
                close: function() {
                    console.log('[VOPECS Desktop] Mock window closed');
                    if (printFrame && printFrame.parentNode) {
                        printFrame.parentNode.removeChild(printFrame);
                        printFrame = null;
                    }
                },
                focus: function() { console.log('[VOPECS Desktop] Mock window focus'); },
                blur: function() {},
                closed: false,
                location: { href: 'about:blank' },
                name: '',
                opener: window,
                innerWidth: 400,
                innerHeight: 600
            };

            return mockWindow;
        }

        // Override window.open
        window.open = function(url, target, features) {
            console.log('[VOPECS Desktop] window.open called:', url, target, features);

            // If empty URL or about:blank - this is typically for print popups
            if (!url || url === '' || url === 'about:blank') {
                console.log('[VOPECS Desktop] Creating mock print window');
                return createPrintWindow();
            }

            // If it's a javascript: URL, ignore
            if (url.startsWith('javascript:')) {
                return null;
            }

            // Convert relative URLs to absolute
            let absoluteUrl = url;
            try {
                if (!url.startsWith('http://') && !url.startsWith('https://') && !url.startsWith('file://') && !url.startsWith('blob:') && !url.startsWith('data:')) {
                    absoluteUrl = new URL(url, window.location.href).href;
                }
            } catch(e) {
                absoluteUrl = url;
            }

            // Parse features for width/height
            let width = 900;
            let height = 700;
            if (features) {
                const widthMatch = features.match(/width=(\d+)/);
                const heightMatch = features.match(/height=(\d+)/);
                if (widthMatch) width = parseInt(widthMatch[1]);
                if (heightMatch) height = parseInt(heightMatch[1]);
            }

            // Try Tauri IPC to open a real popup
            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                console.log('[VOPECS Desktop] Opening popup via Tauri:', absoluteUrl);
                window.__TAURI_INTERNALS__.invoke('open_popup_window', {
                    url: absoluteUrl,
                    title: target || null,
                    width: width,
                    height: height
                }).then(label => {
                    console.log('[VOPECS Desktop] Popup opened:', label);
                }).catch(err => {
                    console.error('[VOPECS Desktop] Popup failed:', err);
                });
            }

            // Return a mock window for compatibility
            return createPrintWindow();
        };

        // Handle links with target="_blank"
        document.addEventListener('click', function(e) {
            let el = e.target;
            while (el && el.tagName !== 'A') {
                el = el.parentElement;
            }
            if (el && el.tagName === 'A' && (el.target === '_blank' || el.getAttribute('target') === '_blank')) {
                console.log('[VOPECS Desktop] Intercepted _blank link:', el.href);
                e.preventDefault();
                e.stopPropagation();
                if (el.href) {
                    window.open(el.href, '_blank');
                }
            }
        }, true);

        console.log('[VOPECS Desktop] Popup and print handlers initialized');
    }

    // Initialize only once
    if (!window.__VOPECS_INITIALIZED__) {
        window.__VOPECS_INITIALIZED__ = true;
        initPopupHandlers();
    }
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
