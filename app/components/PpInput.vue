<script setup lang="ts">
defineProps<{
  modelValue: string | number
  label?: string
  type?: string
  placeholder?: string
  hint?: string
}>()
defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <label class="pp-field">
    <span v-if="label" class="pp-field__label">{{ label }}</span>
    <input
      :type="type ?? 'text'"
      class="pp-field__input"
      :value="modelValue"
      :placeholder="placeholder"
      data-tauri-drag-region-exclude
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <span v-if="hint" class="pp-field__hint">{{ hint }}</span>
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
.pp-field__input {
  padding: 0.55rem 0.75rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
  color: var(--color-text);
  font-size: 0.9rem;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.pp-field__input:focus {
  outline: none;
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--color-accent-muted);
}
.pp-field__hint {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}
</style>
