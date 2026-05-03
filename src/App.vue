<template>
    <main class="app-shell">
        <div class="tabs" role="tablist" :aria-label="t('tabs.listLabel')">
            <button
                v-for="documentItem in documents"
                :key="documentItem.id"
                type="button"
                class="tab-button"
                :class="{ active: documentItem.id === activeDocumentId }"
                role="tab"
                :aria-selected="documentItem.id === activeDocumentId"
                @click="activateDocument(documentItem.id)"
            >
                <span class="tab-title">{{ formatTabTitle(documentItem) }}</span>
                <span class="tab-close" @click.stop="closeDocument(documentItem.id)">x</span>
            </button>
        </div>

        <section v-if="activeDocument" class="editor-workspace">
            <section class="pane">
                <MilkdownEditor
                    :key="activeDocument.id"
                    :model-value="activeDocument.content"
                    :placeholder="t('editor.placeholder')"
                    @update:model-value="updateActiveContent"
                />
            </section>
        </section>
    </main>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, onMounted, ref, watch, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import {
    applyFontMode,
    applyLocaleMode,
    applyThemeMode,
    type FontMode,
    type LocaleCode,
    type LocaleMode,
    type ThemeMode,
} from "./i18n";
import MilkdownEditor from "./components/MilkdownEditor.vue";

type DocumentTab = {
    id: string;
    name: string;
    path: string | null;
    content: string;
    dirty: boolean;
};

type OpenedDocument = {
    name: string;
    path: string;
    content: string;
};

type SavedDocument = {
    name: string;
    path: string;
};

type TypographyStylesheet = {
    path: string;
    content: string;
};

const LOCALE_MENU_EVENT = "language-menu-selected";
const THEME_MENU_EVENT = "theme-menu-selected";
const FONT_MENU_EVENT = "font-menu-selected";
const APP_MENU_EVENT = "app-menu-selected";
const TYPOGRAPHY_STYLE_TAG_ID = "custom-typography-stylesheet";
const { t } = useI18n();
const localeMode = ref<LocaleMode>("auto");
const themeMode = ref<ThemeMode>("system");
const fontMode = ref<FontMode>("system");
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const documents = ref<DocumentTab[]>([]);
const activeDocumentId = ref<string>("");
const untitledCounter = ref(1);
const typographyStylesheetPath = ref<string | null>(null);

const activeDocument = computed(
    () => documents.value.find((documentItem) => documentItem.id === activeDocumentId.value) ?? null
);

function syncAppState(localeCode: LocaleCode) {
    if (!isTauri) {
        return Promise.resolve();
    }

    return invoke("sync_app_state", {
        localeMode: localeMode.value,
        locale: localeCode,
        themeMode: themeMode.value,
        fontMode: fontMode.value,
    });
}

function applyCurrentTheme() {
    return applyThemeMode(themeMode.value);
}

function applyTypographyStylesheet(cssContent: string) {
    if (typeof document === "undefined") {
        return;
    }

    const existingTag = document.getElementById(TYPOGRAPHY_STYLE_TAG_ID);
    if (!cssContent.trim()) {
        existingTag?.remove();
        return;
    }

    const styleTag = existingTag ?? document.createElement("style");
    styleTag.id = TYPOGRAPHY_STYLE_TAG_ID;
    styleTag.textContent = cssContent;

    if (!existingTag) {
        document.head.append(styleTag);
    }
}

function makeDocumentId() {
    return `${Date.now()}-${crypto.randomUUID()}`;
}

function formatUntitledName() {
    const name = t("tabs.untitled", { n: untitledCounter.value });
    untitledCounter.value += 1;
    return name;
}

function createDocument(partial?: Partial<DocumentTab>) {
    return {
        id: makeDocumentId(),
        name: partial?.name ?? formatUntitledName(),
        path: partial?.path ?? null,
        content: partial?.content ?? "",
        dirty: partial?.dirty ?? false,
    } satisfies DocumentTab;
}

function activateDocument(id: string) {
    activeDocumentId.value = id;
}

function createNewDocument() {
    const documentItem = createDocument();
    documents.value = [...documents.value, documentItem];
    activeDocumentId.value = documentItem.id;
}

function updateActiveContent(nextMarkdown: string) {
    const currentDocument = activeDocument.value;
    if (!currentDocument) {
        return;
    }

    currentDocument.content = nextMarkdown;
    currentDocument.dirty = true;
}

function formatTabTitle(documentItem: DocumentTab) {
    return documentItem.dirty ? `* ${documentItem.name}` : documentItem.name;
}

function closeDocument(id: string) {
    const target = documents.value.find((documentItem) => documentItem.id === id);
    if (!target) {
        return;
    }

    if (target.dirty && !window.confirm(t("tabs.confirmCloseDirty", { name: target.name }))) {
        return;
    }

    const nextDocuments = documents.value.filter((documentItem) => documentItem.id !== id);
    documents.value = nextDocuments;

    if (activeDocumentId.value === id) {
        activeDocumentId.value = nextDocuments.at(-1)?.id ?? "";
    }
}

async function openDocuments() {
    if (!isTauri) {
        return;
    }

    const openedDocuments = await invoke<OpenedDocument[]>("open_markdown_files");
    if (openedDocuments.length === 0) {
        return;
    }

    const nextDocuments = [...documents.value];
    let nextActiveId = "";

    for (const openedDocument of openedDocuments) {
        const existingDocument = nextDocuments.find(
            (documentItem) => documentItem.path === openedDocument.path
        );

        if (existingDocument) {
            existingDocument.name = openedDocument.name;
            existingDocument.content = openedDocument.content;
            existingDocument.dirty = false;
            nextActiveId ||= existingDocument.id;
            continue;
        }

        const documentItem = createDocument({
            name: openedDocument.name,
            path: openedDocument.path,
            content: openedDocument.content,
        });
        nextDocuments.push(documentItem);
        nextActiveId ||= documentItem.id;
    }

    documents.value = nextDocuments;
    if (nextActiveId) {
        activeDocumentId.value = nextActiveId;
    }
}

async function saveActiveDocument(saveAs: boolean) {
    const currentDocument = activeDocument.value;
    if (!currentDocument || !isTauri) {
        return;
    }

    const savedDocument = await invoke<SavedDocument | null>("save_document", {
        path: saveAs ? null : currentDocument.path,
        suggestedName: currentDocument.name,
        content: currentDocument.content,
    });

    if (!savedDocument) {
        return;
    }

    currentDocument.name = savedDocument.name;
    currentDocument.path = savedDocument.path;
    currentDocument.dirty = false;
}

async function handleMenuAction(actionId: string) {
    switch (actionId) {
        case "file_new":
            createNewDocument();
            break;
        case "file_open":
            await openDocuments();
            break;
        case "file_save":
            await saveActiveDocument(false);
            break;
        case "file_save_as":
            await saveActiveDocument(true);
            break;
        case "preferences_select_typography_css":
            await selectTypographyStylesheet();
            break;
        case "preferences_clear_typography_css":
            await clearTypographyStylesheet();
            break;
        default:
            break;
    }
}

async function selectTypographyStylesheet() {
    if (!isTauri) {
        return;
    }

    const stylesheet = await invoke<TypographyStylesheet | null>("select_typography_stylesheet");
    if (!stylesheet) {
        return;
    }

    typographyStylesheetPath.value = stylesheet.path;
    applyTypographyStylesheet(stylesheet.content);
}

async function clearTypographyStylesheet() {
    if (!isTauri) {
        return;
    }

    await invoke("clear_typography_stylesheet");
    typographyStylesheetPath.value = null;
    applyTypographyStylesheet("");
}

watch([localeMode, themeMode, fontMode], ([nextLocaleMode]) => {
    const resolvedLocale = applyLocaleMode(nextLocaleMode);
    applyCurrentTheme();
    applyFontMode(fontMode.value);
    void syncAppState(resolvedLocale);
});

watchEffect(() => {
    const currentDocument = activeDocument.value;
    document.title = currentDocument
        ? `${formatTabTitle(currentDocument)} - ${t("app.name")}`
        : t("app.name");
});

let unlistenLanguageMenuEvent: (() => void) | null = null;
let unlistenThemeMenuEvent: (() => void) | null = null;
let unlistenFontMenuEvent: (() => void) | null = null;
let unlistenAppMenuEvent: (() => void) | null = null;
let colorSchemeMedia: MediaQueryList | null = null;
let handleColorSchemeChange: ((event: MediaQueryListEvent) => void) | null = null;

onMounted(async () => {
    if (!isTauri) {
        applyLocaleMode(localeMode.value);
        applyCurrentTheme();
        applyFontMode(fontMode.value);
        return;
    }

    const savedLocaleMode = await invoke<LocaleMode>("get_saved_locale_mode");
    const savedThemeMode = await invoke<ThemeMode>("get_saved_theme_mode");
    const savedFontMode = await invoke<FontMode>("get_saved_font_mode");
    const savedTypographyStylesheet =
        await invoke<TypographyStylesheet | null>("get_saved_typography_stylesheet");
    localeMode.value = savedLocaleMode;
    themeMode.value = savedThemeMode;
    fontMode.value = savedFontMode;
    const resolvedLocale = applyLocaleMode(savedLocaleMode);
    applyCurrentTheme();
    applyFontMode(savedFontMode);
    typographyStylesheetPath.value = savedTypographyStylesheet?.path ?? null;
    applyTypographyStylesheet(savedTypographyStylesheet?.content ?? "");

    unlistenLanguageMenuEvent = await listen<LocaleMode>(LOCALE_MENU_EVENT, (event) => {
        if (event.payload === "auto" || event.payload === "en" || event.payload === "zh-CN") {
            localeMode.value = event.payload;
        }
    });

    unlistenThemeMenuEvent = await listen<ThemeMode>(THEME_MENU_EVENT, (event) => {
        if (event.payload === "system" || event.payload === "light" || event.payload === "dark") {
            themeMode.value = event.payload;
        }
    });

    unlistenFontMenuEvent = await listen<FontMode>(FONT_MENU_EVENT, (event) => {
        if (
            event.payload === "system" ||
            event.payload === "serif" ||
            event.payload === "rounded" ||
            event.payload === "mono"
        ) {
            fontMode.value = event.payload;
        }
    });

    unlistenAppMenuEvent = await listen<string>(APP_MENU_EVENT, (event) => {
        void handleMenuAction(event.payload);
    });

    if (typeof window !== "undefined" && typeof window.matchMedia === "function") {
        colorSchemeMedia = window.matchMedia("(prefers-color-scheme: dark)");
        handleColorSchemeChange = () => {
            if (themeMode.value === "system") {
                applyCurrentTheme();
            }
        };
        colorSchemeMedia.addEventListener("change", handleColorSchemeChange);
    }

    await syncAppState(resolvedLocale);
    await invoke("notify_frontend_ready");
});

onBeforeUnmount(() => {
    unlistenLanguageMenuEvent?.();
    unlistenThemeMenuEvent?.();
    unlistenFontMenuEvent?.();
    unlistenAppMenuEvent?.();

    if (colorSchemeMedia && handleColorSchemeChange) {
        colorSchemeMedia.removeEventListener("change", handleColorSchemeChange);
    }
});
</script>

<style scoped>
.app-shell {
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    background: var(--shell-overlay), var(--shell-bg);
    color: var(--text-main);
    font-family: var(--app-font-ui);
}

.tabs {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    border-bottom: 1px solid var(--tabs-border);
    padding: 0 12px;
    background: var(--tabs-bg);
}

.tab-button {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    border: 0;
    border-radius: 10px 10px 0 0;
    padding: 10px 14px 9px;
    background: transparent;
    color: var(--tab-text);
    font: 13px/1.2 var(--app-font-ui);
    cursor: pointer;
}

.tab-button.active {
    background: var(--tab-active-bg);
    color: var(--tab-active-text);
    box-shadow: inset 0 -2px 0 var(--tab-accent);
}

.tab-title {
    overflow: hidden;
    max-width: 220px;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.tab-close {
    color: var(--tab-close);
    font-size: 13px;
    line-height: 1;
}

.tab-button:hover .tab-close {
    color: var(--tab-active-text);
}

.editor-workspace {
    display: flex;
    min-height: 0;
    flex: 1;
    padding: 8px;
}

.pane {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--panel-border);
    border-radius: 10px;
    background: var(--panel-bg);
}
</style>
