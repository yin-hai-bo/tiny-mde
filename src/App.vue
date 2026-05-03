<template>
    <main class="app-shell">
        <header class="app-header">
            <div class="header-top">
                <div>
                    <h1>{{ t("app.name") }}</h1>
                    <p>{{ t("app.description") }}</p>
                </div>
                <p class="locale-mode">{{ languageModeText }}</p>
            </div>
        </header>
        <section class="editor-panel">
            <label class="editor-label" for="markdown-source">{{ t("editor.label") }}</label>
            <textarea
                id="markdown-source"
                v-model="content"
                class="editor-textarea"
                spellcheck="false"
            />
        </section>
    </main>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
    applyLocaleMode,
    readSavedLocaleMode,
    resolveLocaleFromMode,
    type LocaleCode,
    type LocaleMode,
} from "./i18n";

const { locale, t } = useI18n();
const localeMode = ref<LocaleMode>(readSavedLocaleMode());
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const samples: Record<LocaleCode, string> = {
    en: `# Tiny Markdown Editor

This is the first runnable version.

- Vue is mounted
- Vite can build the frontend
- The editor area is ready for Milkdown later
`,
    "zh-CN": `# Tiny Markdown Editor

这是第一个可以运行的版本。

- Vue 已成功挂载
- Vite 可以正常构建前端
- 编辑区已经准备好，后面再接入 Milkdown
`,
};

const content = ref(samples[resolveLocaleFromMode(localeMode.value)]);
const languageModeText = computed(() =>
    t("app.languageMode", {
        mode:
            localeMode.value === "auto"
                ? t("app.languageModeAuto")
                : localeMode.value === "zh-CN"
                  ? t("app.languageModeChinese")
                  : t("app.languageModeEnglish"),
    })
);

function syncMenuState(mode: LocaleMode, localeCode: LocaleCode) {
    if (!isTauri) {
        return;
    }

    void invoke("sync_menu_state", { mode, locale: localeCode });
}

watch(
    localeMode,
    (nextMode) => {
        const previousSamples = Object.values(samples);
        const resolvedLocale = applyLocaleMode(nextMode);

        if (previousSamples.includes(content.value)) {
            content.value = samples[resolvedLocale];
        }

        syncMenuState(nextMode, resolvedLocale);
    },
    { immediate: true }
);

let unlistenMenuEvent: (() => void) | null = null;

onMounted(async () => {
    if (!isTauri) {
        return;
    }

    unlistenMenuEvent = await listen<LocaleMode>("language-menu-selected", (event) => {
        if (event.payload === "auto" || event.payload === "en" || event.payload === "zh-CN") {
            localeMode.value = event.payload;
        }
    });
});

onBeforeUnmount(() => {
    unlistenMenuEvent?.();
});
</script>

<style scoped>
.app-shell {
    min-height: 100vh;
    padding: 32px;
    background: #f5f1e8;
    color: #1f1a17;
    font-family: "Segoe UI", sans-serif;
}

.app-header {
    margin-bottom: 20px;
}

.header-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
}

.app-header h1 {
    margin: 0 0 8px;
    font-size: 32px;
}

.app-header p {
    margin: 0;
    color: #5a514a;
}

.locale-mode {
    margin: 0;
    padding: 8px 12px;
    border: 1px solid #cbbba7;
    border-radius: 999px;
    background: #fffdf8;
    color: #5a514a;
    white-space: nowrap;
}

.editor-panel {
    display: grid;
    gap: 8px;
}

.editor-label {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
}

.editor-textarea {
    min-height: 320px;
    padding: 16px;
    border: 1px solid #cbbba7;
    border-radius: 12px;
    background: #fffdf8;
    color: inherit;
    font: 15px/1.6 "Cascadia Code", "Consolas", monospace;
    resize: vertical;
}

.editor-textarea:focus {
    outline: 2px solid #8c5e3c;
    outline-offset: 2px;
}

@media (max-width: 720px) {
    .header-top {
        flex-direction: column;
    }

    .locale-mode {
        white-space: normal;
    }
}
</style>
