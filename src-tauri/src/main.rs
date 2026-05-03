#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{
        AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder,
        PredefinedMenuItem, SubmenuBuilder,
    },
    AppHandle, Emitter, Manager, Wry,
};

const FILE_NEW_ID: &str = "file_new";
const FILE_OPEN_ID: &str = "file_open";
const FILE_SAVE_ID: &str = "file_save";
const FILE_SAVE_AS_ID: &str = "file_save_as";
const LANGUAGE_AUTO_ID: &str = "language_auto";
const LANGUAGE_EN_ID: &str = "language_en";
const LANGUAGE_ZH_CN_ID: &str = "language_zh_cn";
const LANGUAGE_MENU_EVENT: &str = "language-menu-selected";

#[tauri::command]
fn sync_menu_state(
    app: AppHandle<Wry>,
    mode: String,
    locale: String,
) -> Result<(), String> {
    if !matches!(mode.as_str(), "auto" | "en" | "zh-CN") {
        return Err("unsupported language mode".to_string());
    }

    if !matches!(locale.as_str(), "en" | "zh-CN") {
        return Err("unsupported locale".to_string());
    }

    let menu = build_app_menu(&app, mode.as_str(), locale.as_str()).map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
    Ok(())
}

fn emit_language_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(LANGUAGE_MENU_EVENT, mode);
    }
}

fn is_chinese_locale(locale: &str) -> bool {
    locale == "zh-CN"
}

fn build_language_submenu(
    app: &AppHandle<Wry>,
    mode: &str,
    locale: &str,
) -> tauri::Result<tauri::menu::Submenu<Wry>> {
    let auto = CheckMenuItemBuilder::with_id(LANGUAGE_AUTO_ID, "自动(&A)")
        .checked(false)
        .build(app)?;
    let english = CheckMenuItemBuilder::with_id(LANGUAGE_EN_ID, "英文(&E)")
        .checked(false)
        .build(app)?;
    let simplified_chinese =
        CheckMenuItemBuilder::with_id(LANGUAGE_ZH_CN_ID, "简体中文(&S)")
            .checked(false)
            .build(app)?;

    auto.set_checked(false)?;
    english.set_checked(false)?;
    simplified_chinese.set_checked(false)?;
    match mode {
        "auto" => auto.set_checked(true)?,
        "zh-CN" => simplified_chinese.set_checked(true)?,
        _ => english.set_checked(true)?,
    }

    let language_submenu = SubmenuBuilder::new(
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
        .build()?;

    Ok(language_submenu)
}

fn build_app_menu(app: &AppHandle<Wry>, mode: &str, locale: &str) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let is_chinese = is_chinese_locale(locale);
    let language_submenu = build_language_submenu(app, mode, locale)?;

    let new_item = MenuItemBuilder::with_id(
        FILE_NEW_ID,
        if is_chinese { "新建(&N)" } else { "&New" },
    )
        .accelerator("CmdOrCtrl+N")
        .enabled(false)
        .build(app)?;
    let open_item = MenuItemBuilder::with_id(
        FILE_OPEN_ID,
        if is_chinese { "打开(&O)..." } else { "&Open..." },
    )
        .accelerator("CmdOrCtrl+O")
        .enabled(false)
        .build(app)?;
    let save_item = MenuItemBuilder::with_id(
        FILE_SAVE_ID,
        if is_chinese { "保存(&S)" } else { "&Save" },
    )
        .accelerator("CmdOrCtrl+S")
        .enabled(false)
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id(
        FILE_SAVE_AS_ID,
        if is_chinese { "另存为(&A)..." } else { "Save &As..." },
    )
        .accelerator("CmdOrCtrl+Shift+S")
        .enabled(false)
        .build(app)?;

    let preferences_submenu = SubmenuBuilder::new(
        app,
        if is_chinese { "首选项(&P)" } else { "&Preferences" },
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

    let about_metadata = AboutMetadataBuilder::new()
        .version(Some("0.1.0"))
        .build();
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

    let menu = MenuBuilder::new(app)
        .item(&file_submenu)
        .item(&edit_submenu)
        .item(&view_submenu)
        .item(&help_submenu)
        .build()?;

    Ok(menu)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let menu = build_app_menu(&app.handle(), "auto", "en")?;
            app.set_menu(menu)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![sync_menu_state])
        .on_menu_event(|app, event| {
            let mode = match event.id().as_ref() {
                LANGUAGE_AUTO_ID => "auto",
                LANGUAGE_EN_ID => "en",
                LANGUAGE_ZH_CN_ID => "zh-CN",
                _ => return,
            };

            emit_language_change(app, mode);
        })
        .run(tauri::generate_context!())
        .expect("error while running Tiny Markdown Editor");
}
