<script setup lang="ts">
import {
  Squares2X2Icon,
  CommandLineIcon,
  PuzzlePieceIcon,
  Cog6ToothIcon,
} from '@heroicons/vue/24/outline'

const route = useRoute()
const id = computed(() => route.params.id as string)

const links = computed(() => [
  { to: `/networks/${id.value}`, label: 'Overview', icon: Squares2X2Icon, exact: true },
  { to: `/networks/${id.value}/settings`, label: 'Settings', icon: Cog6ToothIcon, exact: false },
  { to: `/networks/${id.value}/plugins`, label: 'Plugins', icon: PuzzlePieceIcon, exact: false },
  { to: `/networks/${id.value}/console`, label: 'Console', icon: CommandLineIcon, exact: false },
])

function isActive(link: { to: string; exact: boolean }) {
  if (link.exact) return route.path === link.to
  return route.path === link.to || route.path.startsWith(`${link.to}/`)
}
</script>

<template>
  <nav class="subnav" aria-label="Network sections">
    <NuxtLink
      v-for="link in links"
      :key="link.to"
      :to="link.to"
      class="subnav__link"
      :class="{ 'subnav__link--active': isActive(link) }"
    >
      <component :is="link.icon" class="subnav__icon" aria-hidden="true" />
      {{ link.label }}
    </NuxtLink>
  </nav>
</template>

<style scoped>
.subnav {
  display: flex;
  gap: 0.35rem;
  margin-bottom: 1.25rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid var(--color-border-subtle);
  overflow-x: auto;
  flex-shrink: 0;
}
.subnav__link {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.85rem;
  border-radius: var(--radius-md);
  text-decoration: none;
  color: var(--color-text-muted);
  font-size: 0.85rem;
  font-weight: 600;
  transition: all 0.2s ease;
  white-space: nowrap;
}
.subnav__link:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.subnav__link--active {
  color: var(--color-accent-hover);
  background: var(--color-accent-muted);
}
.subnav__icon {
  width: 1.1rem;
  height: 1.1rem;
}
</style>
