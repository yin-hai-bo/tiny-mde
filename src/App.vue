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
            <div class="document-meta">
                <div>
                    <h1>{{ activeDocument.name }}</h1>
                    <p>{{ activeDocument.path ?? t("tabs.unsaved") }}</p>
                </div>
                <p class="document-state">
                    {{ activeDocument.dirty ? t("tabs.modified") : t("tabs.saved") }}
                </p>
            </div>

            <textarea
                id="markdown-source"
                class="editor-textarea"
                spellcheck="false"
                :value="activeDocument.content"
                @input="updateActiveContent"
            />
        </section>
    </main>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, onMounted, ref, watch, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import { applyLocaleMode, type LocaleCode, type LocaleMode } from "./i18n";

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

const MENU_EVENT = "app-menu-selected";
const { t } = useI18n();
const localeMode = ref<LocaleMode>("auto");
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const documents = ref<DocumentTab[]>([]);
const activeDocumentId = ref<string>("");
const untitledCounter = ref(1);

const activeDocument = computed(
    () => documents.value.find((documentItem) => documentItem.id === activeDocumentId.value) ?? null
);

function syncMenuState(mode: LocaleMode, localeCode: LocaleCode) {
    if (!isTauri) {
        return Promise.resolve();
    }

    return invoke("sync_menu_state", { mode, locale: localeCode });
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

function updateActiveContent(event: Event) {
    const currentDocument = activeDocument.value;
    if (!currentDocument) {
        return;
    }

    currentDocument.content = (event.target as HTMLTextAreaElement).value;
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

    const canRecycleInitialDocument =
        documents.value.length === 1 &&
        !documents.value[0].path &&
        !documents.value[0].dirty &&
        documents.value[0].content.length === 0;

    const nextDocuments = [...documents.value];
    let recycleConsumed = false;
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

        if (canRecycleInitialDocument && !recycleConsumed) {
            const recycledDocument = nextDocuments[0];
            recycledDocument.name = openedDocument.name;
            recycledDocument.path = openedDocument.path;
            recycledDocument.content = openedDocument.content;
            recycledDocument.dirty = false;
            recycleConsumed = true;
            nextActiveId ||= recycledDocument.id;
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

watch(
    localeMode,
    (nextMode) => {
        const resolvedLocale = applyLocaleMode(nextMode);
        void syncMenuState(nextMode, resolvedLocale);
    }
);

watchEffect(() => {
    const currentDocument = activeDocument.value;
    document.title = currentDocument
        ? `${formatTabTitle(currentDocument)} - ${t("app.name")}`
        : t("app.name");
});

let unlistenLanguageMenuEvent: (() => void) | null = null;
let unlistenAppMenuEvent: (() => void) | null = null;

onMounted(async () => {
    if (!isTauri) {
        applyLocaleMode(localeMode.value);
        return;
    }

    const savedLocaleMode = await invoke<LocaleMode>("get_saved_locale_mode");
    localeMode.value = savedLocaleMode;
    const resolvedLocale = applyLocaleMode(savedLocaleMode);

    unlistenLanguageMenuEvent = await listen<LocaleMode>("language-menu-selected", (event) => {
        if (event.payload === "auto" || event.payload === "en" || event.payload === "zh-CN") {
            localeMode.value = event.payload;
        }
    });

    unlistenAppMenuEvent = await listen<string>(MENU_EVENT, (event) => {
        void handleMenuAction(event.payload);
    });

    await syncMenuState(localeMode.value, resolvedLocale);
});

onBeforeUnmount(() => {
    unlistenLanguageMenuEvent?.();
    unlistenAppMenuEvent?.();
});

</script>

<style scoped>
:global(html),
:global(body),
:global(#app) {
    margin: 0;
    min-height: 100%;
    background: #161a21;
}

.app-shell {
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    background:
        linear-gradient(180deg, #1e232d 0%, #161a21 100%),
        linear-gradient(135deg, rgba(255, 255, 255, 0.04), transparent 60%);
    color: #e9edf5;
    font-family: "Segoe UI", sans-serif;
}

.tabs {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    border-bottom: 1px solid #343b48;
    padding: 0 12px;
    background: #1a1f28;
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
    color: #b8c0cf;
    font: inherit;
    cursor: pointer;
}

.tab-button.active {
    background: #242b36;
    color: #ffffff;
    box-shadow: inset 0 -2px 0 #d7a44c;
}

.tab-title {
    overflow: hidden;
    max-width: 220px;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.tab-close {
    color: #8993a7;
    font-size: 16px;
    line-height: 1;
}

.tab-button:hover .tab-close {
    color: #ffffff;
}

.editor-workspace {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    padding: 18px;
    gap: 16px;
}

.document-meta {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
}

.document-meta h1 {
    margin: 0 0 6px;
    font-size: 22px;
    font-weight: 600;
}

.document-meta p {
    margin: 0;
    color: #98a2b7;
}

.document-state {
    border: 1px solid #495266;
    border-radius: 999px;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 13px;
    white-space: nowrap;
}

.editor-textarea {
    min-height: 0;
    flex: 1;
    border: 1px solid #394252;
    border-radius: 16px;
    padding: 18px 20px;
    background: #0e1117;
    color: inherit;
    font: 16px/1.75 "Cascadia Code", "Consolas", monospace;
    resize: none;
}

.editor-textarea:focus {
    outline: 2px solid #d7a44c;
    outline-offset: 2px;
}

@media (max-width: 720px) {
    .document-meta {
        flex-direction: column;
        align-items: stretch;
    }

    .document-state {
        white-space: normal;
    }
}
</style>
