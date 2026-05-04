#![windows_subsystem = "windows"]

use rfd::FileDialog;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{
    menu::{
        AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder,
        PredefinedMenuItem, SubmenuBuilder,
    },
    AppHandle, Emitter, Manager, State, Wry,
};

const DEFAULT_LOCALE_MODE: &str = "auto";
const DEFAULT_THEME_MODE: &str = "system";
const DEFAULT_FONT_MODE: &str = "system";
const FILE_NEW_ID: &str = "file_new";
const FILE_OPEN_ID: &str = "file_open";
const FILE_SAVE_ID: &str = "file_save";
const FILE_SAVE_AS_ID: &str = "file_save_as";
const LANGUAGE_AUTO_ID: &str = "language_auto";
const LANGUAGE_EN_ID: &str = "language_en";
const LANGUAGE_ZH_CN_ID: &str = "language_zh_cn";
const THEME_SYSTEM_ID: &str = "theme_system";
const THEME_LIGHT_ID: &str = "theme_light";
const THEME_DARK_ID: &str = "theme_dark";
const FONT_SYSTEM_ID: &str = "font_system";
const FONT_SERIF_ID: &str = "font_serif";
const FONT_ROUNDED_ID: &str = "font_rounded";
const FONT_MONO_ID: &str = "font_mono";
const TYPOGRAPHY_SELECT_ID: &str = "preferences_select_typography_css";
const TYPOGRAPHY_CLEAR_ID: &str = "preferences_clear_typography_css";
const LANGUAGE_MENU_EVENT: &str = "language-menu-selected";
const THEME_MENU_EVENT: &str = "theme-menu-selected";
const FONT_MENU_EVENT: &str = "font-menu-selected";
const APP_MENU_EVENT: &str = "app-menu-selected";
const LOCALE_MODE_FILE_NAME: &str = "locale-mode.txt";
const THEME_MODE_FILE_NAME: &str = "theme-mode.txt";
const FONT_MODE_FILE_NAME: &str = "font-mode.txt";
const TYPOGRAPHY_CSS_FILE_NAME: &str = "typography-css-path.txt";

#[derive(Default)]
struct AppRuntimeState {
    frontend_ready: bool,
    pending_menu_actions: Vec<String>,
}

struct SharedAppState {
    runtime: Mutex<AppRuntimeState>,
}

#[derive(Serialize)]
struct OpenedDocument {
    name: String,
    path: String,
    content: String,
}

#[derive(Serialize)]
struct SavedDocument {
    name: String,
    path: String,
}

#[derive(Serialize)]
struct TypographyStylesheet {
    path: String,
    content: String,
}

#[tauri::command]
fn get_saved_locale_mode(app: AppHandle<Wry>) -> Result<String, String> {
    load_saved_locale_mode(&app)
}

#[tauri::command]
fn get_saved_theme_mode(app: AppHandle<Wry>) -> Result<String, String> {
    load_saved_theme_mode(&app)
}

#[tauri::command]
fn get_saved_font_mode(app: AppHandle<Wry>) -> Result<String, String> {
    load_saved_font_mode(&app)
}

#[tauri::command]
fn get_saved_typography_stylesheet(
    app: AppHandle<Wry>,
) -> Result<Option<TypographyStylesheet>, String> {
    load_saved_typography_stylesheet(&app)
}

#[tauri::command]
fn sync_app_state(
    app: AppHandle<Wry>,
    locale_mode: String,
    locale: String,
    theme_mode: String,
    font_mode: String,
) -> Result<(), String> {
    if !matches!(locale_mode.as_str(), "auto" | "en" | "zh-CN") {
        return Err("unsupported language mode".to_string());
    }

    if !matches!(locale.as_str(), "en" | "zh-CN") {
        return Err("unsupported locale".to_string());
    }

    if !matches!(theme_mode.as_str(), "system" | "light" | "dark") {
        return Err("unsupported theme mode".to_string());
    }

    if !matches!(font_mode.as_str(), "system" | "serif" | "rounded" | "mono") {
        return Err("unsupported font mode".to_string());
    }

    save_locale_mode(&app, locale_mode.as_str())?;
    save_theme_mode(&app, theme_mode.as_str())?;
    save_font_mode(&app, font_mode.as_str())?;

    let menu = build_app_menu(
        &app,
        locale_mode.as_str(),
        locale.as_str(),
        theme_mode.as_str(),
        font_mode.as_str(),
    )
    .map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn notify_frontend_ready(app: AppHandle<Wry>, state: State<'_, SharedAppState>) -> Result<(), String> {
    let pending_actions = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "failed to lock app runtime state".to_string())?;
        runtime.frontend_ready = true;
        std::mem::take(&mut runtime.pending_menu_actions)
    };

    for action_id in pending_actions {
        emit_app_menu_action(&app, action_id.as_str());
    }

    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|error| error.to_string())?;
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
fn open_markdown_files() -> Result<Vec<OpenedDocument>, String> {
    let Some(paths) = markdown_dialog().pick_files() else {
        return Ok(Vec::new());
    };

    paths.into_iter().map(read_document).collect()
}

#[tauri::command]
fn save_document(
    path: Option<String>,
    suggested_name: Option<String>,
    content: String,
) -> Result<Option<SavedDocument>, String> {
    let target_path = match path.filter(|value| !value.is_empty()) {
        Some(existing_path) => PathBuf::from(existing_path),
        None => {
            let mut dialog = markdown_dialog();
            if let Some(name) = suggested_name.as_deref() {
                dialog = dialog.set_file_name(name);
            }

            let Some(selected_path) = dialog.save_file() else {
                return Ok(None);
            };

            selected_path
        }
    };

    fs::write(&target_path, content)
        .map_err(|error| format!("failed to write {}: {error}", target_path.display()))?;

    Ok(Some(SavedDocument {
        name: file_name(&target_path),
        path: target_path.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
fn select_typography_stylesheet(
    app: AppHandle<Wry>,
) -> Result<Option<TypographyStylesheet>, String> {
    let Some(path) = FileDialog::new()
        .add_filter("CSS", &["css"])
        .add_filter("All files", &["*"])
        .pick_file()
    else {
        return Ok(None);
    };

    let stylesheet = read_typography_stylesheet(path)?;
    save_typography_stylesheet_path(&app, Some(stylesheet.path.as_str()))?;
    Ok(Some(stylesheet))
}

#[tauri::command]
fn clear_typography_stylesheet(app: AppHandle<Wry>) -> Result<(), String> {
    save_typography_stylesheet_path(&app, None)
}

fn emit_language_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(LANGUAGE_MENU_EVENT, mode);
    }
}

fn emit_theme_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(THEME_MENU_EVENT, mode);
    }
}

fn emit_font_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(FONT_MENU_EVENT, mode);
    }
}

fn emit_app_menu_action(app: &AppHandle<Wry>, action_id: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(APP_MENU_EVENT, action_id);
    }
}

fn dispatch_or_queue_app_menu_action(
    app: &AppHandle<Wry>,
    state: &SharedAppState,
    action_id: &str,
) -> Result<(), String> {
    let should_emit_now = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "failed to lock app runtime state".to_string())?;

        if runtime.frontend_ready {
            true
        } else {
            runtime.pending_menu_actions.push(action_id.to_string());
            false
        }
    };

    if should_emit_now {
        emit_app_menu_action(app, action_id);
    }

    Ok(())
}

fn markdown_dialog() -> FileDialog {
    FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "txt"])
        .add_filter("All files", &["*"])
}

fn read_typography_stylesheet(path: PathBuf) -> Result<TypographyStylesheet, String> {
    let content =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    Ok(TypographyStylesheet {
        path: path.to_string_lossy().into_owned(),
        content,
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn app_config_file_path(app: &AppHandle<Wry>, file_name: &str) -> Result<PathBuf, String> {
    let app_config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve app config dir: {error}"))?;

    fs::create_dir_all(&app_config_dir).map_err(|error| {
        format!(
            "failed to create app config dir {}: {error}",
            app_config_dir.display()
        )
    })?;

    Ok(app_config_dir.join(file_name))
}

fn load_saved_locale_mode(app: &AppHandle<Wry>) -> Result<String, String> {
    let path = app_config_file_path(app, LOCALE_MODE_FILE_NAME)?;
    let saved_mode = match fs::read_to_string(&path) {
        Ok(content) => content.trim().to_string(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DEFAULT_LOCALE_MODE.to_string(),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    Ok(normalize_locale_mode(saved_mode.as_str()).to_string())
}

fn load_saved_theme_mode(app: &AppHandle<Wry>) -> Result<String, String> {
    let path = app_config_file_path(app, THEME_MODE_FILE_NAME)?;
    let saved_mode = match fs::read_to_string(&path) {
        Ok(content) => content.trim().to_string(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DEFAULT_THEME_MODE.to_string(),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    Ok(normalize_theme_mode(saved_mode.as_str()).to_string())
}

fn load_saved_font_mode(app: &AppHandle<Wry>) -> Result<String, String> {
    let path = app_config_file_path(app, FONT_MODE_FILE_NAME)?;
    let saved_mode = match fs::read_to_string(&path) {
        Ok(content) => content.trim().to_string(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DEFAULT_FONT_MODE.to_string(),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    Ok(normalize_font_mode(saved_mode.as_str()).to_string())
}

fn load_saved_typography_stylesheet(
    app: &AppHandle<Wry>,
) -> Result<Option<TypographyStylesheet>, String> {
    let Some(path) = load_saved_typography_stylesheet_path(app)? else {
        return Ok(None);
    };

    match read_typography_stylesheet(path.clone()) {
        Ok(stylesheet) => Ok(Some(stylesheet)),
        Err(error) if !path.exists() => {
            save_typography_stylesheet_path(app, None)?;
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn load_saved_typography_stylesheet_path(app: &AppHandle<Wry>) -> Result<Option<PathBuf>, String> {
    let path = app_config_file_path(app, TYPOGRAPHY_CSS_FILE_NAME)?;
    let saved_path = match fs::read_to_string(&path) {
        Ok(content) => content.trim().to_string(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    if saved_path.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(saved_path)))
}

fn save_locale_mode(app: &AppHandle<Wry>, mode: &str) -> Result<(), String> {
    let path = app_config_file_path(app, LOCALE_MODE_FILE_NAME)?;
    fs::write(&path, normalize_locale_mode(mode))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn save_theme_mode(app: &AppHandle<Wry>, mode: &str) -> Result<(), String> {
    let path = app_config_file_path(app, THEME_MODE_FILE_NAME)?;
    fs::write(&path, normalize_theme_mode(mode))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn save_font_mode(app: &AppHandle<Wry>, mode: &str) -> Result<(), String> {
    let path = app_config_file_path(app, FONT_MODE_FILE_NAME)?;
    fs::write(&path, normalize_font_mode(mode))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn save_typography_stylesheet_path(app: &AppHandle<Wry>, path_value: Option<&str>) -> Result<(), String> {
    let path = app_config_file_path(app, TYPOGRAPHY_CSS_FILE_NAME)?;

    match path_value.filter(|value| !value.is_empty()) {
        Some(value) => fs::write(&path, value)
            .map_err(|error| format!("failed to write {}: {error}", path.display())),
        None => match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
        },
    }
}

fn normalize_locale_mode(mode: &str) -> &str {
    match mode {
        "en" => "en",
        "zh-CN" => "zh-CN",
        _ => DEFAULT_LOCALE_MODE,
    }
}

fn normalize_theme_mode(mode: &str) -> &str {
    match mode {
        "light" => "light",
        "dark" => "dark",
        _ => DEFAULT_THEME_MODE,
    }
}

fn normalize_font_mode(mode: &str) -> &str {
    match mode {
        "serif" => "serif",
        "rounded" => "rounded",
        "mono" => "mono",
        _ => DEFAULT_FONT_MODE,
    }
}

fn resolve_locale_from_mode(mode: &str) -> String {
    match normalize_locale_mode(mode) {
        "en" => "en".to_string(),
        "zh-CN" => "zh-CN".to_string(),
        _ => {
            let system_locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
            if system_locale.starts_with("zh") {
                "zh-CN".to_string()
            } else {
                "en".to_string()
            }
        }
    }
}

fn is_chinese_locale(locale: &str) -> bool {
    locale == "zh-CN"
}

fn read_document(path: PathBuf) -> Result<OpenedDocument, String> {
    let content =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    Ok(OpenedDocument {
        name: file_name(&path),
        path: path.to_string_lossy().into_owned(),
        content,
    })
}

fn build_language_submenu(
    app: &AppHandle<Wry>,
    locale_mode: &str,
    locale: &str,
) -> tauri::Result<tauri::menu::Submenu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let auto = CheckMenuItemBuilder::with_id(
        LANGUAGE_AUTO_ID,
        if is_chinese { "自动(&A)" } else { "Automatic(&A)" },
    )
    .checked(false)
    .build(app)?;
    let english = CheckMenuItemBuilder::with_id(
        LANGUAGE_EN_ID,
        if is_chinese { "英文(&E)" } else { "English(&E)" },
    )
    .checked(false)
    .build(app)?;
    let simplified_chinese = CheckMenuItemBuilder::with_id(
        LANGUAGE_ZH_CN_ID,
        if is_chinese {
            "简体中文(&S)"
        } else {
            "Simplified Chinese(&S)"
        },
    )
    .checked(false)
    .build(app)?;

    match normalize_locale_mode(locale_mode) {
        "auto" => auto.set_checked(true)?,
        "zh-CN" => simplified_chinese.set_checked(true)?,
        _ => english.set_checked(true)?,
    }

    SubmenuBuilder::new(app, if is_chinese { "语言(&L)" } else { "&Language" })
        .item(&auto)
        .item(&english)
        .item(&simplified_chinese)
        .build()
}

fn build_theme_submenu(
    app: &AppHandle<Wry>,
    theme_mode: &str,
    locale: &str,
) -> tauri::Result<tauri::menu::Submenu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let system = CheckMenuItemBuilder::with_id(
        THEME_SYSTEM_ID,
        if is_chinese {
            "跟随系统(&S)"
        } else {
            "Follow System(&S)"
        },
    )
    .checked(false)
    .build(app)?;
    let light = CheckMenuItemBuilder::with_id(
        THEME_LIGHT_ID,
        if is_chinese { "浅色(&L)" } else { "Light(&L)" },
    )
    .checked(false)
    .build(app)?;
    let dark = CheckMenuItemBuilder::with_id(
        THEME_DARK_ID,
        if is_chinese { "深色(&D)" } else { "Dark(&D)" },
    )
    .checked(false)
    .build(app)?;

    match normalize_theme_mode(theme_mode) {
        "light" => light.set_checked(true)?,
        "dark" => dark.set_checked(true)?,
        _ => system.set_checked(true)?,
    }

    SubmenuBuilder::new(app, if is_chinese { "主题(&T)" } else { "&Theme" })
        .item(&system)
        .item(&light)
        .item(&dark)
        .build()
}

fn build_font_submenu(
    app: &AppHandle<Wry>,
    font_mode: &str,
    locale: &str,
) -> tauri::Result<tauri::menu::Submenu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let system = CheckMenuItemBuilder::with_id(
        FONT_SYSTEM_ID,
        if is_chinese {
            "系统无衬线(&S)"
        } else {
            "System Sans(&S)"
        },
    )
    .checked(false)
    .build(app)?;
    let serif = CheckMenuItemBuilder::with_id(
        FONT_SERIF_ID,
        if is_chinese { "衬线文稿(&E)" } else { "Serif Editorial(&E)" },
    )
    .checked(false)
    .build(app)?;
    let rounded = CheckMenuItemBuilder::with_id(
        FONT_ROUNDED_ID,
        if is_chinese { "圆润界面(&R)" } else { "Rounded UI(&R)" },
    )
    .checked(false)
    .build(app)?;
    let mono = CheckMenuItemBuilder::with_id(
        FONT_MONO_ID,
        if is_chinese { "等宽打字机(&M)" } else { "Monospace(&M)" },
    )
    .checked(false)
    .build(app)?;

    match normalize_font_mode(font_mode) {
        "serif" => serif.set_checked(true)?,
        "rounded" => rounded.set_checked(true)?,
        "mono" => mono.set_checked(true)?,
        _ => system.set_checked(true)?,
    }

    SubmenuBuilder::new(app, if is_chinese { "字体风格(&F)" } else { "&Font Style" })
        .item(&system)
        .item(&serif)
        .item(&rounded)
        .item(&mono)
        .build()
}

fn build_app_menu(
    app: &AppHandle<Wry>,
    locale_mode: &str,
    locale: &str,
    theme_mode: &str,
    font_mode: &str,
) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let language_submenu = build_language_submenu(app, locale_mode, locale)?;
    let theme_submenu = build_theme_submenu(app, theme_mode, locale)?;
    let font_submenu = build_font_submenu(app, font_mode, locale)?;

    let new_item = MenuItemBuilder::with_id(FILE_NEW_ID, if is_chinese { "新建(&N)" } else { "&New" })
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_item = MenuItemBuilder::with_id(
        FILE_OPEN_ID,
        if is_chinese { "打开(&O)..." } else { "&Open..." },
    )
    .accelerator("CmdOrCtrl+O")
    .build(app)?;
    let save_item = MenuItemBuilder::with_id(FILE_SAVE_ID, if is_chinese { "保存(&S)" } else { "&Save" })
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id(
        FILE_SAVE_AS_ID,
        if is_chinese {
            "另存为(&A)..."
        } else {
            "Save &As..."
        },
    )
    .accelerator("CmdOrCtrl+Shift+S")
    .build(app)?;

    let preferences_submenu = SubmenuBuilder::new(
        app,
        if is_chinese {
            "首选项(&P)"
        } else {
            "&Preferences"
        },
    )
    .item(&language_submenu)
    .item(&theme_submenu)
    .item(&font_submenu)
    .separator()
    .item(
        &MenuItemBuilder::with_id(
            TYPOGRAPHY_SELECT_ID,
            if is_chinese {
                "选择排版样式表(&Y)..."
            } else {
                "Choose Typography CSS(&Y)..."
            },
        )
        .build(app)?,
    )
    .item(
        &MenuItemBuilder::with_id(
            TYPOGRAPHY_CLEAR_ID,
            if is_chinese {
                "清除排版样式表(&C)"
            } else {
                "Clear Typography CSS(&C)"
            },
        )
        .build(app)?,
    )
    .build()?;

    let file_submenu = SubmenuBuilder::new(app, if is_chinese { "文件(&F)" } else { "&File" })
        .item(&new_item)
        .item(&open_item)
        .separator()
        .item(&save_item)
        .item(&save_as_item)
        .separator()
        .item(&preferences_submenu)
        .separator()
        .item(&PredefinedMenuItem::quit(
            app,
            Some(if is_chinese { "退出(&X)" } else { "E&xit" }),
        )?)
        .build()?;

    let edit_submenu = SubmenuBuilder::new(app, if is_chinese { "编辑(&E)" } else { "&Edit" })
        .undo_with_text(if is_chinese { "撤销(&U)" } else { "&Undo" })
        .redo_with_text(if is_chinese { "重做(&R)" } else { "&Redo" })
        .separator()
        .cut_with_text(if is_chinese { "剪切(&T)" } else { "Cu&t" })
        .copy_with_text(if is_chinese { "复制(&C)" } else { "&Copy" })
        .paste_with_text(if is_chinese { "粘贴(&P)" } else { "&Paste" })
        .separator()
        .select_all_with_text(if is_chinese { "全选(&A)" } else { "Select &All" })
        .build()?;

    let view_submenu = SubmenuBuilder::new(app, if is_chinese { "视图(&V)" } else { "&View" })
        .fullscreen_with_text(if is_chinese { "全屏(&F)" } else { "&Fullscreen" })
        .build()?;

    let about_metadata = AboutMetadataBuilder::new().version(Some("0.1.0")).build();
    let help_submenu = SubmenuBuilder::new(app, if is_chinese { "帮助(&H)" } else { "&Help" })
        .item(&PredefinedMenuItem::about(
            app,
            Some(if is_chinese {
                "关于 Tiny Markdown Editor(&A)"
            } else {
                "&About Tiny Markdown Editor"
            }),
            Some(about_metadata),
        )?)
        .build()?;

    MenuBuilder::new(app)
        .item(&file_submenu)
        .item(&edit_submenu)
        .item(&view_submenu)
        .item(&help_submenu)
        .build()
}

fn main() {
    tauri::Builder::default()
        .manage(SharedAppState {
            runtime: Mutex::new(AppRuntimeState::default()),
        })
        .setup(|app| {
            let locale_mode = load_saved_locale_mode(&app.handle())?;
            let locale = resolve_locale_from_mode(locale_mode.as_str());
            let theme_mode = load_saved_theme_mode(&app.handle())?;
            let font_mode = load_saved_font_mode(&app.handle())?;
            let menu = build_app_menu(
                &app.handle(),
                locale_mode.as_str(),
                locale.as_str(),
                theme_mode.as_str(),
                font_mode.as_str(),
            )
            .map_err(|error| error.to_string())?;
            app.set_menu(menu).map_err(|error| error.to_string())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_saved_locale_mode,
            get_saved_theme_mode,
            get_saved_font_mode,
            get_saved_typography_stylesheet,
            sync_app_state,
            notify_frontend_ready,
            open_markdown_files,
            save_document,
            select_typography_stylesheet,
            clear_typography_stylesheet
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            LANGUAGE_AUTO_ID => emit_language_change(app, "auto"),
            LANGUAGE_EN_ID => emit_language_change(app, "en"),
            LANGUAGE_ZH_CN_ID => emit_language_change(app, "zh-CN"),
            THEME_SYSTEM_ID => emit_theme_change(app, "system"),
            THEME_LIGHT_ID => emit_theme_change(app, "light"),
            THEME_DARK_ID => emit_theme_change(app, "dark"),
            FONT_SYSTEM_ID => emit_font_change(app, "system"),
            FONT_SERIF_ID => emit_font_change(app, "serif"),
            FONT_ROUNDED_ID => emit_font_change(app, "rounded"),
            FONT_MONO_ID => emit_font_change(app, "mono"),
            FILE_NEW_ID
            | FILE_OPEN_ID
            | FILE_SAVE_ID
            | FILE_SAVE_AS_ID
            | TYPOGRAPHY_SELECT_ID
            | TYPOGRAPHY_CLEAR_ID => {
                if let Some(state) = app.try_state::<SharedAppState>() {
                    let _ = dispatch_or_queue_app_menu_action(app, &state, event.id().as_ref());
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Tiny Markdown Editor");
}
