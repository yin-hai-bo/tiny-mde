import { createI18n } from "vue-i18n";

export type LocaleCode = "en" | "zh-CN";
export type LocaleMode = "auto" | LocaleCode;

const DEFAULT_LOCALE: LocaleCode = "en";

const messages = {
    en: {
        app: {
            name: "Tiny Markdown Editor",
        },
        tabs: {
            listLabel: "Open documents",
            untitled: "Untitled {n}",
            unsaved: "Unsaved document",
            saved: "Saved",
            modified: "Modified",
            confirmCloseDirty: 'Close "{name}" without saving?',
        },
    },
    "zh-CN": {
        app: {
            name: "Tiny Markdown Editor",
        },
        tabs: {
            listLabel: "已打开文档",
            untitled: "未命名 {n}",
            unsaved: "未保存文档",
            saved: "已保存",
            modified: "已修改",
            confirmCloseDirty: '关闭“{name}”且不保存吗？',
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

export const i18n = createI18n({
    legacy: false,
    locale: DEFAULT_LOCALE,
    fallbackLocale: DEFAULT_LOCALE,
    messages,
});

export function applyLocaleMode(mode: LocaleMode) {
    const locale = resolveLocaleFromMode(mode);

    i18n.global.locale.value = locale;

    if (typeof document !== "undefined") {
        document.documentElement.lang = locale;
        document.title = i18n.global.t("app.name");
    }

    return locale;
}
