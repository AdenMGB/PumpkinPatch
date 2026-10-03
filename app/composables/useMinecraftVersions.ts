import { useQuery } from '@tanstack/vue-query'

export function useMinecraftVersions() {
  const api = usePatchApi()
  const showSnapshots = ref(false)
  const search = ref('')

  const { data, isLoading, isError, refetch } = useQuery({
    queryKey: ['minecraft-versions'],
    queryFn: () => api.minecraftListVersions(),
    staleTime: 1000 * 60 * 60,
  })

  const filtered = computed(() => {
    let list = data.value ?? []
    if (!showSnapshots.value) {
      list = list.filter((v) => v.stable)
    }
    const q = search.value.trim().toLowerCase()
    if (q) {
      list = list.filter((v) => v.version.toLowerCase().includes(q))
    }
    return list
  })

  const selectOptions = computed(() =>
    filtered.value.map((v) => ({
      value: v.version,
      label: v.stable ? v.version : `${v.version} (snapshot)`,
    })),
  )

  return {
    showSnapshots,
    search,
    selectOptions,
    allVersions: data,
    isLoading,
    isError,
    refetch,
  }
}
