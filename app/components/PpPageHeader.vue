<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <header class="pp-page-header">
    <div>
      <h1>{{ title }}</h1>
      <p v-if="description" class="pp-muted">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="pp-page-header__actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.pp-page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 1.5rem;
  flex-wrap: wrap;
}
h1 {
  margin: 0 0 0.35rem;
  font-size: 1.65rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.pp-page-header__actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  align-items: center;
}
</style>
