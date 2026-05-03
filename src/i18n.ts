import { createI18n } from "vue-i18n";

export type LocaleCode = "en" | "zh-CN";
export type LocaleMode = "auto" | LocaleCode;

const DEFAULT_LOCALE: LocaleCode = "en";
const STORAGE_KEY = "tiny-mde.locale-mode";

const messages = {
    en: {
        app: {
            name: "Tiny Markdown Editor",
            description: "A minimal runnable Markdown editor shell built with Vue.",
            languageMenu: "Language",
            languageMode: "Current language mode: {mode}",
            languageModeAuto: "Automatic",
            languageModeEnglish: "English",
            languageModeChinese: "Simplified Chinese",
        },
        editor: {
            label: "Markdown",
        },
    },
    "zh-CN": {
        app: {
            name: "Tiny Markdown Editor",
            description: "一个基于 Vue 的最小可运行 Markdown 编辑器外壳。",
            languageMenu: "语言",
            languageMode: "当前语言模式：{mode}",
            languageModeAuto: "自动",
            languageModeEnglish: "英文",
            languageModeChinese: "简体中文",
        },
        editor: {
            label: "Markdown",
        },
    },
} as const;

function resolveSystemLocale(): LocaleCode {
    if (typeof navigator === "undefined") {
        return DEFAULT_LOCALE;
    }

    return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function resolveLocaleFromMode(mode: LocaleMode): LocaleCode {
    return mode === "auto" ? resolveSystemLocale() : mode;
}

export function readSavedLocaleMode(): LocaleMode {
    if (typeof localStorage === "undefined") {
        return "auto";
    }

    const savedMode = localStorage.getItem(STORAGE_KEY);
    return savedMode === "en" || savedMode === "zh-CN" || savedMode === "auto"
        ? savedMode
        : "auto";
}

export const i18n = createI18n({
    legacy: false,
    locale: resolveLocaleFromMode(readSavedLocaleMode()),
    fallbackLocale: DEFAULT_LOCALE,
    messages,
});

export function applyLocaleMode(mode: LocaleMode) {
    const locale = resolveLocaleFromMode(mode);

    i18n.global.locale.value = locale;

    if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, mode);
    }

    if (typeof document !== "undefined") {
        document.documentElement.lang = locale;
        document.title = i18n.global.t("app.name");
    }

    return locale;
}
