import { createI18n } from "vue-i18n";

export type LocaleCode = "en" | "zh-CN";
export type LocaleMode = "auto" | LocaleCode;
export type ThemeMode = "system" | "light" | "dark";
export type FontMode = "system" | "serif" | "rounded" | "mono";
export type ResolvedTheme = "light" | "dark";

const DEFAULT_LOCALE: LocaleCode = "en";

const messages = {
    en: {
        app: {
            name: "Tiny Markdown Editor",
        },
        editor: {
            placeholder: "\u5728\u8fd9\u91cc\u8f93\u5165 Markdown...",
        },
        sidebar: {
            title: "\u4fa7\u8fb9\u680f",
            placeholder: "\u5927\u7eb2\u4f1a\u663e\u793a\u5728\u8fd9\u91cc\u3002",
        },
        tabs: {
            listLabel: "\u5df2\u6253\u5f00\u6587\u6863",
            untitled: "\u672a\u547d\u540d {n}",
            confirmCloseDirty: "\u5173\u95ed\u201c{name}\u201d\u4e14\u4e0d\u4fdd\u5b58\u5417\uff1f",
        },
    },
    "zh-CN": {
        app: {
            name: "Tiny Markdown Editor",
        },
        editor: {
            placeholder: "Write markdown here...",
        },
        sidebar: {
            title: "Sidebar",
            placeholder: "Outline will appear here.",
        },
        tabs: {
            listLabel: "Open documents",
            untitled: "Untitled {n}",
            confirmCloseDirty: 'Close "{name}" without saving?',
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

export function applyFontMode(mode: FontMode) {
    if (typeof document !== "undefined") {
        document.documentElement.dataset.fontMode = mode;
    }

    return mode;
}
