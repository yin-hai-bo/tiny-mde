<template>
    <div ref="rootElement" class="milkdown-host" />
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
    "update:modelValue": [value: string];
}>();

const rootElement = ref<HTMLDivElement | null>(null);
let crepe: Crepe | null = null;
let lastSyncedMarkdown = props.modelValue;

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
    await createEditor(props.modelValue);
});

onBeforeUnmount(() => {
    void destroyEditor();
});
</script>

<style scoped>
.milkdown-host {
    min-height: 0;
    flex: 1;
    overflow: auto;
}
</style>
