<script setup lang="ts">
import {
  Squares2X2Icon,
  PlusCircleIcon,
  Cog6ToothIcon,
} from '@heroicons/vue/24/outline'

const links = [
  { to: '/library', label: 'Networks', icon: Squares2X2Icon },
  { to: '/networks/new', label: 'Create', icon: PlusCircleIcon },
  { to: '/settings', label: 'Settings', icon: Cog6ToothIcon },
]
</script>

<template>
  <aside class="sidebar" aria-label="Main navigation">
    <div class="sidebar__brand" data-tauri-drag-region>
      <span class="sidebar__logo" aria-hidden="true">🎃</span>
      <span class="sidebar__name">Pumpkin Patch</span>
    </div>
    <nav class="sidebar__nav">
      <NuxtLink
        v-for="link in links"
        :key="link.to"
        :to="link.to"
        class="sidebar__link"
        active-class="sidebar__link--active"
      >
        <component :is="link.icon" class="sidebar__icon" aria-hidden="true" />
        <span>{{ link.label }}</span>
      </NuxtLink>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: var(--sidebar-width);
  padding: 1rem 0.75rem;
  border-right: 1px solid var(--color-border-subtle);
  background: var(--color-bg-raised);
  min-height: 0;
}
.sidebar__brand {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.25rem 0.5rem 1.25rem;
}
.sidebar__logo {
  font-size: 1.35rem;
}
.sidebar__name {
  font-weight: 700;
  font-size: 0.95rem;
  letter-spacing: -0.02em;
}
.sidebar__nav {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.sidebar__link {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.55rem 0.75rem;
  border-radius: var(--radius-md);
  color: var(--color-text-muted);
  text-decoration: none;
  font-size: 0.875rem;
  font-weight: 600;
  transition: all 0.2s ease;
}
.sidebar__link:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
  transform: scale(1.02);
}
.sidebar__link--active {
  color: var(--color-accent-hover);
  background: var(--color-accent-muted);
}
.sidebar__icon {
  width: 1.25rem;
  height: 1.25rem;
  flex-shrink: 0;
}
</style>
