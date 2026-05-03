<template>
    <div ref="rootElement" class="milkdown-host markdown-typography" />
</template>

<script setup lang="ts">
import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{
    modelValue: string;
    placeholder: string;
}>();

const emit = defineEmits<{
    edited: [];
    "update:modelValue": [value: string];
}>();

const rootElement = ref<HTMLDivElement | null>(null);
let crepe: Crepe | null = null;
let lastSyncedMarkdown = props.modelValue;
let suppressNextInputEvent = false;

async function destroyEditor() {
    if (!crepe) {
        return;
    }

    const instance = crepe;
    crepe = null;
    await instance.destroy();
}

async function createEditor(initialMarkdown: string) {
    await nextTick();
    if (!rootElement.value) {
        return;
    }

    suppressNextInputEvent = true;

    const instance = new Crepe({
        root: rootElement.value,
        defaultValue: initialMarkdown,
        featureConfigs: {
            [Crepe.Feature.Placeholder]: {
                text: props.placeholder,
                mode: "doc",
            },
        },
    });

    instance.on((listener) => {
        listener.markdownUpdated((_ctx, markdown) => {
            lastSyncedMarkdown = markdown;
            emit("update:modelValue", markdown);
        });
    });

    await instance.create();
    crepe = instance;
    lastSyncedMarkdown = initialMarkdown;
}

async function rebuildEditor(markdown: string) {
    await destroyEditor();
    await createEditor(markdown);
}

function handleEditorMutation() {
    if (suppressNextInputEvent) {
        suppressNextInputEvent = false;
        return;
    }

    emit("edited");
}

watch(
    () => props.modelValue,
    async (nextValue) => {
        if (!crepe || nextValue === lastSyncedMarkdown) {
            return;
        }

        await rebuildEditor(nextValue);
    }
);

watch(
    () => props.placeholder,
    async () => {
        if (!crepe) {
            return;
        }

        await rebuildEditor(props.modelValue);
    }
);

onMounted(async () => {
    rootElement.value?.addEventListener("beforeinput", handleEditorMutation);
    rootElement.value?.addEventListener("input", handleEditorMutation);
    await createEditor(props.modelValue);
});

onBeforeUnmount(() => {
    rootElement.value?.removeEventListener("beforeinput", handleEditorMutation);
    rootElement.value?.removeEventListener("input", handleEditorMutation);
    void destroyEditor();
});
</script>

<style scoped>
.milkdown-host {
    min-height: 0;
    flex: 1;
    overflow: hidden;
}
</style>
