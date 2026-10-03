<script setup lang="ts">
import { useMutation, useQuery } from '@tanstack/vue-query'

const api = usePatchApi()
const router = useRouter()
const step = ref(1)
const javaStatus = ref<string | null>(null)

const { data: settings } = useQuery({
  queryKey: ['settings'],
  queryFn: () => api.settingsGet(),
})

const validateJava = useMutation({
  mutationFn: () => api.javaValidate(),
  onSuccess: (msg) => {
    javaStatus.value = msg
    step.value = 2
  },
  onError: (e: Error) => {
    javaStatus.value = e.message
  },
})

const name = ref('My Network')
const createMutation = useMutation({
  mutationFn: () =>
    api.networkCreate({
      name: name.value,
      backend_names: ['survival'],
      hub_type: 'velocity',
    }),
  onSuccess: (summary) => {
    step.value = 3
    router.push(`/networks/${summary.network.id}`)
  },
})
</script>

<template>
  <div class="pp-page onboarding">
    <PpPageHeader
      title="Welcome to Pumpkin Patch"
      description="Set up Java, create a network, and start playing."
    />
    <PpCard padding="lg">
      <ol class="steps">
        <li :class="{ active: step === 1, done: step > 1 }">
          <h2>Java</h2>
          <p class="pp-muted">JDK {{ settings?.java_min_major ?? 21 }}+ required for Velocity and Pumpkin.</p>
          <PpButton :loading="validateJava.isPending.value" @click="validateJava.mutate()">
            Validate Java
          </PpButton>
          <p v-if="javaStatus" class="status">{{ javaStatus }}</p>
        </li>
        <li :class="{ active: step === 2, done: step > 2 }">
          <h2>Create network</h2>
          <PpInput v-model="name" label="Network name" />
          <PpButton :loading="createMutation.isPending.value" @click="createMutation.mutate()">
            Create &amp; open
          </PpButton>
        </li>
        <li :class="{ active: step === 3 }">
          <h2>Play</h2>
          <p class="pp-muted">Start the network from the overview, copy the join address, and connect in Minecraft.</p>
        </li>
      </ol>
    </PpCard>
  </div>
</template>

<style scoped>
.onboarding {
  max-width: 36rem;
}
.steps {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
.steps li {
  opacity: 0.55;
}
.steps li.active,
.steps li.done {
  opacity: 1;
}
.steps h2 {
  margin: 0 0 0.35rem;
  font-size: 1rem;
}
.status {
  margin-top: 0.5rem;
  font-size: 0.85rem;
}
</style>
