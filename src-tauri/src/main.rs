#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, SubmenuBuilder},
    AppHandle, Emitter, Manager, State, Wry,
};

const LANGUAGE_AUTO_ID: &str = "language_auto";
const LANGUAGE_EN_ID: &str = "language_en";
const LANGUAGE_ZH_CN_ID: &str = "language_zh_cn";
const LANGUAGE_MENU_EVENT: &str = "language-menu-selected";

struct LanguageMenuState {
    auto: CheckMenuItem<Wry>,
    english: CheckMenuItem<Wry>,
    simplified_chinese: CheckMenuItem<Wry>,
}

impl LanguageMenuState {
    fn set_checked(&self, mode: &str) -> tauri::Result<()> {
        self.auto.set_checked(mode == "auto")?;
        self.english.set_checked(mode == "en")?;
        self.simplified_chinese.set_checked(mode == "zh-CN")?;
        Ok(())
    }
}

#[tauri::command]
fn set_language_menu_state(
    state: State<'_, LanguageMenuState>,
    mode: String,
) -> Result<(), String> {
    if !matches!(mode.as_str(), "auto" | "en" | "zh-CN") {
        return Err("unsupported language mode".to_string());
    }

    state
        .set_checked(mode.as_str())
        .map_err(|error| error.to_string())
}

fn emit_language_change(app: &AppHandle<Wry>, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(LANGUAGE_MENU_EVENT, mode);
    }
}

fn build_language_menu(app: &AppHandle<Wry>) -> tauri::Result<(LanguageMenuState, tauri::menu::Menu<Wry>)> {
    let auto = CheckMenuItemBuilder::with_id(LANGUAGE_AUTO_ID, "Auto / 自动")
        .checked(true)
        .build(app)?;
    let english = CheckMenuItemBuilder::with_id(LANGUAGE_EN_ID, "English")
        .checked(false)
        .build(app)?;
    let simplified_chinese =
        CheckMenuItemBuilder::with_id(LANGUAGE_ZH_CN_ID, "简体中文")
            .checked(false)
            .build(app)?;

    let language_submenu = SubmenuBuilder::new(app, "Language / 语言")
        .item(&auto)
        .item(&english)
        .item(&simplified_chinese)
        .build()?;
    let menu = MenuBuilder::new(app).item(&language_submenu).build()?;

    Ok((
        LanguageMenuState {
            auto,
            english,
            simplified_chinese,
        },
        menu,
    ))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let (language_menu_state, menu) = build_language_menu(&app.handle())?;
            app.manage(language_menu_state);
            app.set_menu(menu)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_language_menu_state])
        .on_menu_event(|app, event| {
            let mode = match event.id().as_ref() {
                LANGUAGE_AUTO_ID => "auto",
                LANGUAGE_EN_ID => "en",
                LANGUAGE_ZH_CN_ID => "zh-CN",
                _ => return,
            };

            if let Some(state) = app.try_state::<LanguageMenuState>() {
                let _ = state.set_checked(mode);
            }

            emit_language_change(app, mode);
        })
        .run(tauri::generate_context!())
        .expect("error while running Tiny Markdown Editor");
}
