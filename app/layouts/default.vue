<template>
  <div class="app-grid-layout">
    <AppSidebar />
    <div class="app-main-column">
      <header class="app-grid-statusbar" data-tauri-drag-region>
        <div class="statusbar-leading" data-tauri-drag-region>
          <span class="pp-muted statusbar-tag">Server network hub</span>
        </div>
      </header>
      <main class="app-contents">
        <slot />
      </main>
    </div>
    <AppWindowControls />
  </div>
</template>

<style scoped>
.app-grid-layout {
  display: grid;
  grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
  flex: 1;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  background: var(--color-bg-raised);
  font-family:
    'Segoe UI',
    system-ui,
    -apple-system,
    sans-serif;
}
.app-main-column {
  display: grid;
  grid-template-rows: var(--top-bar-height) minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.app-grid-statusbar {
  display: flex;
  align-items: center;
  padding: 0 0.5rem 0 1rem;
  padding-right: calc(0.5rem + var(--window-controls-width, 0px));
  border-bottom: 1px solid var(--color-border-subtle);
  background: var(--color-bg-raised);
}
.statusbar-leading {
  display: flex;
  align-items: center;
  flex: 1;
}
.statusbar-tag {
  font-size: 0.8rem;
  user-select: none;
}
.app-contents {
  min-height: 0;
  height: 100%;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
  background: var(--color-bg);
  border-top-left-radius: var(--radius-xl);
  padding: 1.75rem 2rem 2.5rem;
}
</style>
