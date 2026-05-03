import { createI18n } from "vue-i18n";

export type LocaleCode = "en" | "zh-CN";
export type LocaleMode = "auto" | LocaleCode;
export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const DEFAULT_LOCALE: LocaleCode = "en";

const messages = {
    en: {
        app: {
            name: "Tiny Markdown Editor",
        },
        editor: {
            editor: "Editor",
            placeholder: "Write markdown here...",
        },
        tabs: {
            listLabel: "Open documents",
            untitled: "Untitled {n}",
            confirmCloseDirty: 'Close "{name}" without saving?',
        },
    },
    "zh-CN": {
        app: {
            name: "Tiny Markdown Editor",
        },
        editor: {
            editor: "编辑",
            placeholder: "在这里输入 Markdown...",
        },
        tabs: {
            listLabel: "已打开文档",
            untitled: "未命名 {n}",
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

function prefersDarkTheme() {
    return typeof window !== "undefined" &&
        typeof window.matchMedia === "function" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveLocaleFromMode(mode: LocaleMode): LocaleCode {
    return mode === "auto" ? resolveSystemLocale() : mode;
}

export function resolveThemeFromMode(mode: ThemeMode): ResolvedTheme {
    if (mode === "light" || mode === "dark") {
        return mode;
    }

    return prefersDarkTheme() ? "dark" : "light";
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

export function applyThemeMode(mode: ThemeMode) {
    const theme = resolveThemeFromMode(mode);

    if (typeof document !== "undefined") {
        document.documentElement.dataset.theme = theme;
        document.documentElement.style.colorScheme = theme;
    }

    return theme;
}
