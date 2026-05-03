#![windows_subsystem = "windows"]

use rfd::FileDialog;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{
    menu::{
        AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder,
        PredefinedMenuItem, SubmenuBuilder,
    },
    AppHandle, Emitter, Manager, Wry,
};

const DEFAULT_LOCALE_MODE: &str = "auto";
const FILE_NEW_ID: &str = "file_new";
const FILE_OPEN_ID: &str = "file_open";
const FILE_SAVE_ID: &str = "file_save";
const FILE_SAVE_AS_ID: &str = "file_save_as";
const LANGUAGE_AUTO_ID: &str = "language_auto";
const LANGUAGE_EN_ID: &str = "language_en";
const LANGUAGE_ZH_CN_ID: &str = "language_zh_cn";
const LANGUAGE_MENU_EVENT: &str = "language-menu-selected";
const APP_MENU_EVENT: &str = "app-menu-selected";
const LOCALE_MODE_FILE_NAME: &str = "locale-mode.txt";

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

#[tauri::command]
fn get_saved_locale_mode(app: AppHandle<Wry>) -> Result<String, String> {
    load_saved_locale_mode(&app)
}

#[tauri::command]
fn sync_menu_state(app: AppHandle<Wry>, mode: String, locale: String) -> Result<(), String> {
    if !matches!(mode.as_str(), "auto" | "en" | "zh-CN") {
        return Err("unsupported language mode".to_string());
    }

    if !matches!(locale.as_str(), "en" | "zh-CN") {
        return Err("unsupported locale".to_string());
    }

    save_locale_mode(&app, mode.as_str())?;

    let menu = build_app_menu(&app, mode.as_str(), locale.as_str()).map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
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

fn emit_language_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(LANGUAGE_MENU_EVENT, mode);
    }
}

fn emit_app_menu_action(app: &AppHandle<Wry>, action_id: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(APP_MENU_EVENT, action_id);
    }
}

fn markdown_dialog() -> FileDialog {
    FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "txt"])
        .add_filter("All files", &["*"])
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn locale_mode_path(app: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let app_config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve app config dir: {error}"))?;

    fs::create_dir_all(&app_config_dir)
        .map_err(|error| format!("failed to create app config dir {}: {error}", app_config_dir.display()))?;

    Ok(app_config_dir.join(LOCALE_MODE_FILE_NAME))
}

fn load_saved_locale_mode(app: &AppHandle<Wry>) -> Result<String, String> {
    let path = locale_mode_path(app)?;
    let saved_mode = match fs::read_to_string(&path) {
        Ok(content) => content.trim().to_string(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DEFAULT_LOCALE_MODE.to_string(),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    Ok(normalize_locale_mode(saved_mode.as_str()).to_string())
}

fn save_locale_mode(app: &AppHandle<Wry>, mode: &str) -> Result<(), String> {
    let path = locale_mode_path(app)?;
    let normalized_mode = normalize_locale_mode(mode);
    fs::write(&path, normalized_mode)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn normalize_locale_mode(mode: &str) -> &str {
    match mode {
        "en" => "en",
        "zh-CN" => "zh-CN",
        _ => DEFAULT_LOCALE_MODE,
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
    mode: &str,
    locale: &str,
) -> tauri::Result<tauri::menu::Submenu<Wry>> {
    let auto = CheckMenuItemBuilder::with_id(
        LANGUAGE_AUTO_ID,
        if is_chinese_locale(locale) {
            "自动(&A)"
        } else {
            "Automatic(&A)"
        },
    )
    .checked(false)
    .build(app)?;
    let english = CheckMenuItemBuilder::with_id(
        LANGUAGE_EN_ID,
        if is_chinese_locale(locale) {
            "英文(&E)"
        } else {
            "English(&E)"
        },
    )
    .checked(false)
    .build(app)?;
    let simplified_chinese = CheckMenuItemBuilder::with_id(
        LANGUAGE_ZH_CN_ID,
        if is_chinese_locale(locale) {
            "简体中文(&S)"
        } else {
            "Simplified Chinese(&S)"
        },
    )
    .checked(false)
    .build(app)?;

    auto.set_checked(false)?;
    english.set_checked(false)?;
    simplified_chinese.set_checked(false)?;
    match normalize_locale_mode(mode) {
        "auto" => auto.set_checked(true)?,
        "zh-CN" => simplified_chinese.set_checked(true)?,
        _ => english.set_checked(true)?,
    }

    SubmenuBuilder::new(
        app,
        if is_chinese_locale(locale) {
            "语言(&L)"
        } else {
            "&Language"
        },
    )
    .item(&auto)
    .item(&english)
    .item(&simplified_chinese)
    .build()
}

fn build_app_menu(app: &AppHandle<Wry>, mode: &str, locale: &str) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let language_submenu = build_language_submenu(app, mode, locale)?;

    let new_item = MenuItemBuilder::with_id(FILE_NEW_ID, if is_chinese { "新建(&N)" } else { "&New" })
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_item =
        MenuItemBuilder::with_id(FILE_OPEN_ID, if is_chinese { "打开(&O)..." } else { "&Open..." })
            .accelerator("CmdOrCtrl+O")
            .build(app)?;
    let save_item = MenuItemBuilder::with_id(FILE_SAVE_ID, if is_chinese { "保存(&S)" } else { "&Save" })
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id(
        FILE_SAVE_AS_ID,
        if is_chinese { "另存为(&A)..." } else { "Save &As..." },
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
        .setup(|app| {
            let saved_mode = load_saved_locale_mode(&app.handle())?;
            let locale = resolve_locale_from_mode(saved_mode.as_str());
            let menu =
                build_app_menu(&app.handle(), saved_mode.as_str(), locale.as_str()).map_err(|error| error.to_string())?;
            app.set_menu(menu).map_err(|error| error.to_string())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_saved_locale_mode,
            sync_menu_state,
            open_markdown_files,
            save_document
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            LANGUAGE_AUTO_ID => emit_language_change(app, "auto"),
            LANGUAGE_EN_ID => emit_language_change(app, "en"),
            LANGUAGE_ZH_CN_ID => emit_language_change(app, "zh-CN"),
            FILE_NEW_ID | FILE_OPEN_ID | FILE_SAVE_ID | FILE_SAVE_AS_ID => {
                emit_app_menu_action(app, event.id().as_ref())
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Tiny Markdown Editor");
}
