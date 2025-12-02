import { defineStore } from 'pinia'
import { ref } from 'vue'

import { TauriAPI } from '../api/tauri'

export const useRcloneStore = defineStore('rclone', () => {
  const loading = ref(false)
  const error = ref<string | undefined>()
  const serviceRunning = ref(false)
  const rcloneAvailable = ref(true)

  const setError = (msg?: string) => (error.value = msg)

  const runWithLoading = async <T>(fn: () => Promise<T>): Promise<T> => {
    loading.value = true
    try {
      return await fn()
    } finally {
      loading.value = false
    }
  }

  async function getRcloneProcessId(): Promise<string | undefined> {
    try {
      const processList = await TauriAPI.process.list()
      return processList.find(p => p.config?.name === 'single_rclone_backend_process')?.id
    } catch (err) {
      console.error('Failed to get Rclone process ID:', err)
    }
  }

  const clearError = () => setError()

  const startRcloneBackend = () =>
    runWithLoading(async () => {
      if (await TauriAPI.rclone.backend.isRunning()) {
        serviceRunning.value = true
      }
      const result = await TauriAPI.rclone.backend.createAndStart()
      if (result) {
        serviceRunning.value = true
      }
    }).catch(err => {
      setError('Failed to start rclone service')
      throw err
    })

  const stopRcloneBackend = () =>
    runWithLoading(async () => {
      const id = await getRcloneProcessId()
      if (!id) {
        serviceRunning.value = false
        return true
      }
      const ok = await TauriAPI.process.stop(id)
      if (ok) serviceRunning.value = false
      return ok
    }).catch(err => {
      setError('Failed to stop rclone service')
      throw err
    })

  const checkRcloneBackendStatus = async () => {
    const running = await TauriAPI.rclone.backend.isRunning().catch(() => false)
    serviceRunning.value = running
    return running
  }

  const checkRcloneAvailable = async () => {
    const available = await TauriAPI.rclone.backend.isAvailable().catch(() => false)
    rcloneAvailable.value = available
    return available
  }

  const init = () => {
    console.log('Initializing Rclone store...')
    checkRcloneAvailable()
  }

  return {
    loading,
    error,
    serviceRunning,
    rcloneAvailable,
    clearError,
    startRcloneBackend,
    stopRcloneBackend,
    checkRcloneBackendStatus,
    checkRcloneAvailable,
    init
  }
})
