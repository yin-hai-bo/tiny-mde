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
    applyLocaleMode,
    applyThemeMode,
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

const LOCALE_MENU_EVENT = "language-menu-selected";
const THEME_MENU_EVENT = "theme-menu-selected";
const APP_MENU_EVENT = "app-menu-selected";
const { t } = useI18n();
const localeMode = ref<LocaleMode>("auto");
const themeMode = ref<ThemeMode>("system");
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const documents = ref<DocumentTab[]>([]);
const activeDocumentId = ref<string>("");
const untitledCounter = ref(1);

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
    });
}

function applyCurrentTheme() {
    return applyThemeMode(themeMode.value);
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
        default:
            break;
    }
}

watch([localeMode, themeMode], ([nextLocaleMode]) => {
    const resolvedLocale = applyLocaleMode(nextLocaleMode);
    applyCurrentTheme();
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
let unlistenAppMenuEvent: (() => void) | null = null;
let colorSchemeMedia: MediaQueryList | null = null;
let handleColorSchemeChange: ((event: MediaQueryListEvent) => void) | null = null;

onMounted(async () => {
    if (!isTauri) {
        applyLocaleMode(localeMode.value);
        applyCurrentTheme();
        return;
    }

    const savedLocaleMode = await invoke<LocaleMode>("get_saved_locale_mode");
    const savedThemeMode = await invoke<ThemeMode>("get_saved_theme_mode");
    localeMode.value = savedLocaleMode;
    themeMode.value = savedThemeMode;
    const resolvedLocale = applyLocaleMode(savedLocaleMode);
    applyCurrentTheme();

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
    unlistenAppMenuEvent?.();

    if (colorSchemeMedia && handleColorSchemeChange) {
        colorSchemeMedia.removeEventListener("change", handleColorSchemeChange);
    }
});
</script>

<style scoped>
:global(html),
:global(body),
:global(#app) {
    margin: 0;
    min-height: 100%;
    background: var(--window-bg);
}

:global(:root) {
    --window-bg: #161a21;
    --shell-bg: linear-gradient(180deg, #1e232d 0%, #161a21 100%);
    --shell-overlay: linear-gradient(135deg, rgba(255, 255, 255, 0.04), transparent 60%);
    --text-main: #e9edf5;
    --text-muted: #a7b0c0;
    --tabs-bg: #1a1f28;
    --tabs-border: #343b48;
    --tab-text: #b8c0cf;
    --tab-active-bg: #242b36;
    --tab-active-text: #ffffff;
    --tab-accent: #d7a44c;
    --tab-close: #8993a7;
    --panel-border: #394252;
    --panel-bg: #0e1117;
    --panel-header-bg: #181d26;
    --panel-header-border: #2e3746;
    --editor-toolbar-bg: #181d26;
}

:global(html[data-theme="light"]) {
    --window-bg: #f4f1ea;
    --shell-bg: linear-gradient(180deg, #f9f7f1 0%, #ece7dd 100%);
    --shell-overlay: linear-gradient(135deg, rgba(255, 255, 255, 0.7), rgba(255, 255, 255, 0) 60%);
    --text-main: #2a241e;
    --text-muted: #6a6158;
    --tabs-bg: #ded7ca;
    --tabs-border: #b8ae9f;
    --tab-text: #605545;
    --tab-active-bg: #fffdf8;
    --tab-active-text: #1f1a17;
    --tab-accent: #9f6b20;
    --tab-close: #7a705f;
    --panel-border: #c8beaf;
    --panel-bg: #fffdf8;
    --panel-header-bg: #f1ebdf;
    --panel-header-border: #d7cdbf;
    --editor-toolbar-bg: #f1ebdf;
}

:global(.milkdown) {
    --crepe-color-background: var(--panel-bg);
    --crepe-color-on-background: var(--text-main);
    --crepe-color-surface: var(--panel-bg);
    --crepe-color-surface-low: var(--panel-header-bg);
    --crepe-color-on-surface: var(--text-main);
    --crepe-color-on-surface-variant: var(--text-muted);
    --crepe-color-outline: var(--panel-border);
    --crepe-color-primary: var(--text-main);
    --crepe-color-secondary: var(--panel-header-border);
    --crepe-color-on-secondary: var(--text-main);
    --crepe-color-inverse: var(--tab-active-bg);
    --crepe-color-on-inverse: var(--tab-active-text);
    --crepe-color-inline-code: #8dc4ff;
    --crepe-color-error: #d24b4b;
    --crepe-color-hover: var(--panel-header-bg);
    --crepe-color-selected: var(--tabs-bg);
    --crepe-color-inline-area: rgba(255, 255, 255, 0.08);
    --crepe-font-title: "Segoe UI", sans-serif;
    --crepe-font-default: "Segoe UI", sans-serif;
    --crepe-font-code: "Cascadia Code", "Consolas", monospace;
    height: 100%;
    background: var(--panel-bg);
    color: var(--text-main);
}

:global(html[data-theme="light"] .milkdown) {
    --crepe-color-inline-code: #0b62b3;
    --crepe-color-inline-area: rgba(74, 58, 38, 0.08);
}

:global(.milkdown .editor) {
    height: 100%;
}

:global(.milkdown .ProseMirror) {
    min-height: 100%;
    padding: 12px 14px 24px;
    color: var(--text-main);
}

:global(.milkdown .toolbar),
:global(.milkdown .milkdown-toolbar) {
    background: var(--editor-toolbar-bg);
}

:global(.milkdown .toolbar button),
:global(.milkdown .milkdown-toolbar button) {
    color: var(--text-main);
}

.app-shell {
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    background: var(--shell-overlay), var(--shell-bg);
    color: var(--text-main);
    font-family: "Segoe UI", sans-serif;
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
    font: 13px/1.2 "Segoe UI", sans-serif;
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
