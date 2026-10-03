<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

const greeting = ref('')
const name = ref('Pumpkin Patch')
const loading = ref(false)
const error = ref<string | null>(null)

async function greet() {
  loading.value = true
  error.value = null
  try {
    greeting.value = await invoke<string>('greet', { name: name.value })
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
    greeting.value = ''
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void greet()
})
</script>

<template>
  <section class="home">
    <h1>Welcome</h1>
    <p class="lede">Nuxt + Tauri v2 — same repo, one desktop app.</p>

    <label class="field">
      <span>Name</span>
      <input v-model="name" type="text" autocomplete="off" />
    </label>

    <button class="btn" type="button" :disabled="loading" @click="greet">
      {{ loading ? 'Calling Rust…' : 'Greet from Rust' }}
    </button>

    <p v-if="greeting" class="greeting">{{ greeting }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.home {
  max-width: 32rem;
}

h1 {
  margin: 0 0 0.5rem;
  font-size: 1.75rem;
}

.lede {
  margin: 0 0 1.5rem;
  color: #b8b8d0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin-bottom: 1rem;
  font-size: 0.9rem;
}

input {
  padding: 0.5rem 0.75rem;
  border-radius: 0.5rem;
  border: 1px solid #3d3d5c;
  background: #12121f;
  color: inherit;
}

.btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 0.5rem;
  background: #e67e22;
  color: #1a1a2e;
  font-weight: 600;
  cursor: pointer;
  transition:
    transform 0.2s ease,
    opacity 0.2s ease;
}

.btn:hover:not(:disabled) {
  transform: scale(1.02);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.greeting {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  background: #22223a;
}

.error {
  margin-top: 1rem;
  color: #ff8a8a;
}
</style>
