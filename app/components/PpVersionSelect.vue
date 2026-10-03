<script setup lang="ts">
const props = defineProps<{
  modelValue: string
  label?: string
  hint?: string
}>()
defineEmits<{ 'update:modelValue': [value: string] }>()

const { selectOptions, showSnapshots, search, isLoading, isError, refetch } =
  useMinecraftVersions()
</script>

<template>
  <div class="version-select">
    <span v-if="label" class="pp-field__label">{{ label }}</span>
    <div class="version-toolbar">
      <input
        v-model="search"
        type="search"
        class="version-search"
        placeholder="Filter versions…"
        data-tauri-drag-region-exclude
      />
      <label class="snap-toggle">
        <input v-model="showSnapshots" type="checkbox" data-tauri-drag-region-exclude />
        Snapshots
      </label>
      <PpButton v-if="isError" size="sm" variant="secondary" type="button" @click="refetch()">
        Retry
      </PpButton>
    </div>
    <select
      class="pp-field__select version-list"
      :value="modelValue"
      :disabled="isLoading || !selectOptions.length"
      data-tauri-drag-region-exclude
      @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option v-if="isLoading" value="" disabled>Loading versions…</option>
      <option v-for="opt in selectOptions" :key="opt.value" :value="opt.value">
        {{ opt.label }}
      </option>
    </select>
    <span v-if="hint" class="pp-field__hint">{{ hint }}</span>
    <span v-else-if="!isLoading" class="pp-field__hint">
      {{ selectOptions.length }} versions from Mojang + Modrinth
    </span>
  </div>
</template>

<style scoped>
.version-select {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.pp-field__label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-muted);
}
.version-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}
.version-search {
  flex: 1;
  min-width: 10rem;
  padding: 0.45rem 0.65rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
  color: var(--color-text);
  font-size: 0.85rem;
}
.snap-toggle {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.8rem;
  color: var(--color-text-muted);
}
.pp-field__select {
  padding: 0.55rem 0.75rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
  color: var(--color-text);
  font-size: 0.9rem;
}
.version-list {
  max-height: 14rem;
}
.pp-field__hint {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}
</style>
