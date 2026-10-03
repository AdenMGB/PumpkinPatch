// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-10-01',
  ssr: false,
  devtools: { enabled: true },
  devServer: {
    port: 1420,
    strictPort: true,
  },
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
    },
  },
  app: {
    head: {
      title: 'Pumpkin Patch',
      meta: [{ name: 'description', content: 'Pumpkin Patch desktop app' }],
    },
  },
})
