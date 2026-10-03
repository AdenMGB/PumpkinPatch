<script setup lang="ts">
import { useMutation } from '@tanstack/vue-query'
import { useRouter } from 'vue-router'
import { MinusCircleIcon, PlusIcon } from '@heroicons/vue/24/outline'

const api = usePatchApi()
const router = useRouter()

const name = ref('My Network')
const hubPort = ref(25565)
const minecraftVersion = ref('1.21.4')
const pumpkinChannel = ref('nightly')
const backends = ref(['survival', 'creative'])

function addBackend() {
  backends.value.push(`world-${backends.value.length + 1}`)
}

function removeBackend(i: number) {
  if (backends.value.length > 1) backends.value.splice(i, 1)
}

const createMutation = useMutation({
  mutationFn: () =>
    api.networkCreate({
      name: name.value,
      hub_port: hubPort.value,
      hub_type: 'velocity',
      backend_names: backends.value.filter((b) => b.trim()),
      pumpkin_channel: pumpkinChannel.value,
      minecraft_version: minecraftVersion.value,
    }),
  onSuccess: (summary) => router.push(`/networks/${summary.network.id}`),
})
</script>

<template>
  <div class="pp-page wizard">
    <PpPageHeader
      title="Create network"
      description="Velocity hub + lobby + backends with aligned proxy secrets."
    />

    <form class="wizard-form" @submit.prevent="createMutation.mutate()">
      <PpCard padding="lg" class="step">
        <h2>Basics</h2>
        <div class="fields">
          <PpInput v-model="name" label="Network name" placeholder="Survival SMP" />
          <PpInput v-model.number="hubPort" label="Hub port (join port)" type="number" />
          <PpVersionSelect
            v-model="minecraftVersion"
            label="Default Minecraft version"
            hint="Applied to every server in this network (change per-server later in Settings)."
          />
          <PpInput
            v-model="pumpkinChannel"
            label="Pumpkin release channel"
            hint="Usually nightly for latest protocol"
          />
        </div>
      </PpCard>

      <PpCard padding="lg" class="step">
        <div class="step-head">
          <h2>Game backends</h2>
          <PpButton type="button" variant="secondary" size="sm" @click="addBackend">
            <PlusIcon class="ico" aria-hidden="true" />
            Add backend
          </PpButton>
        </div>
        <div class="backend-list">
          <div v-for="(b, i) in backends" :key="i" class="backend-row">
            <PpInput v-model="backends[i]" :label="`Backend ${i + 1}`" />
            <PpIconButton
              v-if="backends.length > 1"
              label="Remove backend"
              variant="danger"
              @click="removeBackend(i)"
            >
              <MinusCircleIcon aria-hidden="true" />
            </PpIconButton>
          </div>
        </div>
      </PpCard>

      <div class="submit-row">
        <PpButton type="submit" size="lg" :loading="createMutation.isPending.value">
          Create network
        </PpButton>
      </div>
    </form>
  </div>
</template>

<style scoped>
.wizard-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 40rem;
}
.step h2 {
  margin: 0 0 1rem;
  font-size: 1.05rem;
}
.fields {
  display: grid;
  gap: 1rem;
}
.step-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}
.step-head h2 {
  margin: 0;
}
.backend-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.backend-row {
  display: flex;
  gap: 0.5rem;
  align-items: flex-end;
}
.backend-row .pp-field {
  flex: 1;
}
.ico {
  width: 1rem;
  height: 1rem;
}
.submit-row {
  margin-top: 0.5rem;
}
</style>
