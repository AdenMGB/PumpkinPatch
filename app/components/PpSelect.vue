<script setup lang="ts">
defineProps<{
  modelValue: string
  label?: string
  options: { value: string; label: string }[]
}>()
defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <label class="pp-field">
    <span v-if="label" class="pp-field__label">{{ label }}</span>
    <select
      class="pp-field__select"
      :value="modelValue"
      data-tauri-drag-region-exclude
      @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="opt in options" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
    </select>
  </label>
</template>

<style scoped>
.pp-field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.pp-field__label {
  font-size: 0.8rem;
  font-weight: 600;
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
.pp-field__select:focus {
  outline: none;
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--color-accent-muted);
}
</style>
