import tailwindcss from '@tailwindcss/vite'

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-10-01',
  css: ['~/assets/css/tailwind.css', '~/assets/css/shell.css'],
  ssr: false,
  devtools: { enabled: true },
  devServer: {
    port: 1420,
    strictPort: true,
  },
  vite: {
    plugins: [tailwindcss()],
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
