<script setup lang="ts">
withDefaults(defineProps<{ padding?: 'md' | 'lg'; hover?: boolean }>(), {
  padding: 'md',
  hover: false,
})
</script>

<template>
  <div class="pp-card" :class="[`pp-card--${padding}`, { 'pp-card--hover': hover }]">
    <slot />
  </div>
</template>

<style scoped>
.pp-card {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  transition: border-color 0.2s ease, transform 0.2s ease;
}
.pp-card--md {
  padding: 1rem 1.15rem;
}
.pp-card--lg {
  padding: 1.25rem 1.5rem;
}
.pp-card--hover:hover {
  border-color: var(--color-border);
  transform: translateY(-2px);
}
</style>
