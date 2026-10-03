<template>
  <div class="layout">
    <header class="titlebar">
      <span class="title">Pumpkin Patch</span>
    </header>
    <main class="content">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #1a1a2e;
  color: #f5f5f5;
  font-family:
    system-ui,
    -apple-system,
    'Segoe UI',
    sans-serif;
}

.titlebar {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #2d2d44;
  background: #16162a;
}

.title {
  font-weight: 600;
  letter-spacing: 0.02em;
}

.content {
  flex: 1;
  padding: 1.5rem;
}
</style>
