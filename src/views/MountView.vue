<script setup lang="ts">
import {
  CheckCircle,
  Cloud,
  Edit,
  FolderOpen,
  HardDrive,
  Loader,
  Play,
  Plus,
  RefreshCw,
  Save,
  Search,
  Settings,
  Square,
  Trash2,
  X,
  XCircle
} from 'lucide-vue-next'
import { computed, ComputedRef, onMounted, onUnmounted, Ref, ref } from 'vue'

import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import { useAppStore } from '@/stores/app'

import { useTranslation } from '../composables/useI18n'
import { useRcloneStore } from '../stores/rclone'

const { t } = useTranslation()
const rcloneStore = useRcloneStore()
const appStore = useAppStore()

const showAddForm = ref(false)
const editingConfig = ref<RcloneFormConfig | null>(null)
const searchQuery = ref('')
const statusFilter = ref<'all' | 'mounted' | 'unmounted' | 'error'>('all')

const showConfirmDialog = ref(false)
const confirmDialogConfig = ref({
  title: '',
  message: '',
  configToDelete: null as RcloneFormConfig | null
})

let mountRefreshInterval: NodeJS.Timeout | null = null
let backendStatusCheckInterval: NodeJS.Timeout | null = null

const configForm = ref({
  name: '',
  type: 'webdav',
  url: '',
  vendor: '',
  user: '',
  pass: '',
  mountPoint: '',
  volumeName: '',
  autoMount: false,
  extraFlags: [] as string[],
  extraOptions: {
    'vfs-cache-mode': 'full'
  }
}) as Ref<RcloneFormConfig>

const commonFlags = ref([
  {
    category: 'Caching',
    flags: [
      { flag: '--vfs-cache-mode', value: 'full', descriptionKey: 'vfs-cache-mode-full' },
      { flag: '--vfs-cache-mode', value: 'writes', descriptionKey: 'vfs-cache-mode-writes' },
      { flag: '--vfs-cache-mode', value: 'minimal', descriptionKey: 'vfs-cache-mode-minimal' },
      { flag: '--vfs-cache-max-age', value: '24h', descriptionKey: 'vfs-cache-max-age' },
      { flag: '--vfs-cache-max-size', value: '10G', descriptionKey: 'vfs-cache-max-size' },
      { flag: '--dir-cache-time', value: '5m', descriptionKey: 'dir-cache-time' }
    ]
  },
  {
    category: 'Performance',
    flags: [
      { flag: '--buffer-size', value: '16M', descriptionKey: 'buffer-size-16M' },
      { flag: '--buffer-size', value: '32M', descriptionKey: 'buffer-size-32M' },
      { flag: '--vfs-read-chunk-size', value: '128M', descriptionKey: 'vfs-read-chunk-size' },
      { flag: '--transfers', value: '4', descriptionKey: 'transfers' },
      { flag: '--checkers', value: '8', descriptionKey: 'checkers' }
    ]
  },
  {
    category: 'Bandwidth',
    flags: [
      { flag: '--bwlimit', value: '10M', descriptionKey: 'bwlimit-10M' },
      { flag: '--bwlimit', value: '10M:100M', descriptionKey: 'bwlimit-10M:100M' },
      { flag: '--bwlimit', value: '08:00,512k 18:00,10M 23:00,off', descriptionKey: 'bwlimit-schedule' }
    ]
  },
  {
    category: 'Network',
    flags: [
      { flag: '--timeout', value: '5m', descriptionKey: 'timeout' },
      { flag: '--contimeout', value: '60s', descriptionKey: 'contimeout' },
      { flag: '--low-level-retries', value: '10', descriptionKey: 'low-level-retries' },
      { flag: '--retries', value: '3', descriptionKey: 'retries' }
    ]
  },
  {
    category: 'Security',
    flags: [
      { flag: '--read-only', value: '', descriptionKey: 'read-only' },
      { flag: '--allow-other', value: '', descriptionKey: 'allow-other' },
      { flag: '--allow-root', value: '', descriptionKey: 'allow-root' },
      { flag: '--umask', value: '022', descriptionKey: 'umask' }
    ]
  },
  {
    category: 'WebDAV Specific',
    flags: [
      { flag: '--webdav-headers', value: 'User-Agent,rclone/1.0', descriptionKey: 'webdav-headers' },
      { flag: '--webdav-bearer-token', value: '', descriptionKey: 'webdav-bearer-token' }
    ]
  },
  {
    category: 'Debugging',
    flags: [
      { flag: '--log-level', value: 'INFO', descriptionKey: 'log-level' },
      { flag: '--verbose', value: '', descriptionKey: 'verbose' },
      { flag: '--use-json-log', value: '', descriptionKey: 'use-json-log' },
      { flag: '--progress', value: '', descriptionKey: 'progress' }
    ]
  }
])

const showFlagSelector = ref(false)

const filteredConfigs: ComputedRef<RcloneFormConfig[]> = computed(() => {
  const filtered: RcloneFormConfig[] = []
  const fullRemoteConfigs = appStore.fullRcloneConfigs

  for (const config of fullRemoteConfigs) {
    if (!config) continue

    const matchesSearch = searchQuery.value
      ? config.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
        config.url.toLowerCase().includes(searchQuery.value.toLowerCase())
      : true
    if (!matchesSearch) continue

    const mountInfo = appStore.mountInfos.find(mount => mount.name === config.name)
    const status = mountInfo?.status || 'unmounted'
    const matchesStatus = statusFilter.value === 'all' || status === statusFilter.value

    if (matchesStatus && matchesSearch) {
      filtered.push(config)
    }
  }
  return filtered
})

const configCounts = computed(() => {
  const fullConfigs = appStore.fullRcloneConfigs
  return {
    total: fullConfigs.length,
    mounted: appStore.mountedConfigs.length,
    unmounted: fullConfigs.length - appStore.mountedConfigs.length,
    error: appStore.mountInfos.filter(m => m.status === 'error').length
  }
})

const addNewConfig = () => {
  resetForm()
  showAddForm.value = true
}

const editConfig = (config: RcloneFormConfig) => {
  editingConfig.value = config
  configForm.value = {
    name: config.name,
    type: config.type,
    url: config.url,
    vendor: config.vendor || '',
    user: config.user,
    pass: config.pass,
    mountPoint: config.mountPoint || '',
    volumeName: config.volumeName || '',
    autoMount: config.autoMount,
    extraFlags: config.extraFlags || []
  }
  showAddForm.value = true
}

const saveConfig = async () => {
  if (!configForm.value.name || !configForm.value.url || !configForm.value.user || !configForm.value.pass) {
    console.error(t('mount.messages.fillRequiredFields'))
    return
  }

  try {
    if (editingConfig.value && editingConfig.value.name) {
      await appStore.updateRemoteConfig(editingConfig.value.name, configForm.value.type, {
        name: configForm.value.name,
        type: configForm.value.type,
        url: configForm.value.url,
        vendor: configForm.value.vendor || undefined,
        user: configForm.value.user,
        pass: configForm.value.pass,
        mountPoint: configForm.value.mountPoint || undefined,
        volumeName: configForm.value.volumeName || undefined,
        autoMount: configForm.value.autoMount,
        extraFlags: configForm.value.extraFlags
      })
    } else {
      await appStore.createRemoteConfig(configForm.value.name, configForm.value.type, {
        name: configForm.value.name,
        type: configForm.value.type,
        url: configForm.value.url,
        vendor: configForm.value.vendor || undefined,
        user: configForm.value.user,
        pass: configForm.value.pass,
        mountPoint: configForm.value.mountPoint || undefined,
        volumeName: configForm.value.volumeName || undefined,
        autoMount: configForm.value.autoMount,
        extraFlags: configForm.value.extraFlags
      })
    }
    showAddForm.value = false
    resetForm()
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToSave'))
  }
}

const cancelForm = () => {
  showAddForm.value = false
  resetForm()
}

const resetForm = () => {
  configForm.value = {
    name: '',
    type: 'webdav',
    url: '',
    vendor: '',
    user: '',
    pass: '',
    mountPoint: '',
    volumeName: '',
    autoMount: false,
    extraFlags: []
  }
  editingConfig.value = null
}

const mountConfig = async (config: RcloneFormConfig) => {
  try {
    await appStore.mountRemote(config.name)
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToMount'))
  }
}

const unmountConfig = async (config: RcloneFormConfig) => {
  if (!config.name) return
  try {
    await appStore.unmountRemote(config.name)
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToUnmount'))
  }
}

const deleteConfig = async (config: RcloneFormConfig) => {
  if (!config.name) return

  confirmDialogConfig.value = {
    title: t('mount.messages.confirmDeleteTitle'),
    message: t('mount.messages.confirmDelete', { name: config.name }),
    configToDelete: config
  }
  showConfirmDialog.value = true
}

const confirmDelete = async () => {
  const config = confirmDialogConfig.value.configToDelete
  if (!config || !config.name) return

  try {
    await appStore.deleteRemoteConfig(config.name)
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToDelete'))
  } finally {
    showConfirmDialog.value = false
    confirmDialogConfig.value.configToDelete = null
  }
}

const cancelDelete = () => {
  showConfirmDialog.value = false
  confirmDialogConfig.value.configToDelete = null
}

const startBackend = async () => {
  try {
    await rcloneStore.startRcloneBackend()
    await new Promise(resolve => setTimeout(resolve, 1000))
    await rcloneStore.checkRcloneBackendStatus()
    await appStore.loadRemoteConfigs()
    await appStore.loadMountInfos()
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToStartService'))
  }
}

const stopBackend = async () => {
  try {
    const stopped = await rcloneStore.stopRcloneBackend()
    if (!stopped) {
      throw new Error(t('mount.messages.failedToStopService'))
    }
  } catch (error: any) {
    console.error(error.message || t('mount.messages.failedToStopService'))
  }
}

const getConfigStatus = (config: RcloneFormConfig) => {
  const mountInfo = appStore.mountInfos.find(mount => mount.name === config.name)
  return mountInfo?.status || 'unmounted'
}

const getStatusIcon = (status: string) => {
  switch (status) {
    case 'mounted':
      return CheckCircle
    case 'mounting':
    case 'unmounting':
      return Loader
    case 'error':
      return XCircle
    default:
      return Square
  }
}

const isConfigMounted = (config: RcloneFormConfig) => {
  const status = getConfigStatus(config)
  return status === 'mounted'
}

const isConfigMounting = (config: RcloneFormConfig) => {
  const status = getConfigStatus(config)
  return status === 'mounting' || status === 'unmounting'
}

const addFlag = () => {
  if (!configForm.value.extraFlags) {
    configForm.value.extraFlags = []
  }
  configForm.value.extraFlags.push('')
}

const removeFlag = (index: number) => {
  if (configForm.value.extraFlags) {
    configForm.value.extraFlags.splice(index, 1)
  }
}

const addFlagToConfig = (flag: { flag: string; value: string; descriptionKey: string }) => {
  if (!configForm.value.extraFlags) {
    configForm.value.extraFlags = []
  }

  const flagKey = `${flag.flag}${flag.value ? `=${flag.value}` : ''}`

  if (flag.flag === '--vfs-cache-mode' || flag.flag === '--buffer-size' || flag.flag === '--log-level') {
    const existingIndex = configForm.value.extraFlags.findIndex(existingFlag => existingFlag.startsWith(flag.flag))
    if (existingIndex !== -1) {
      configForm.value.extraFlags.splice(existingIndex, 1)
    }
  }

  if (!configForm.value.extraFlags.includes(flagKey)) {
    configForm.value.extraFlags.push(flagKey)
  }
}

const removeFlagFromConfig = (flag: { flag: string; value: string; descriptionKey: string }) => {
  if (!configForm.value.extraFlags) return

  const flagKey = `${flag.flag}${flag.value ? `=${flag.value}` : ''}`
  const index = configForm.value.extraFlags.indexOf(flagKey)

  if (index !== -1) {
    configForm.value.extraFlags.splice(index, 1)
  }
}

const isFlagInConfig = (flag: { flag: string; value: string; descriptionKey: string }) => {
  if (!configForm.value.extraFlags) return false
  const flagKey = `${flag.flag}${flag.value ? `=${flag.value}` : ''}`
  return configForm.value.extraFlags.includes(flagKey)
}

const toggleFlag = (flag: { flag: string; value: string; descriptionKey: string }) => {
  if (isFlagInConfig(flag)) {
    removeFlagFromConfig(flag)
  } else {
    addFlagToConfig(flag)
  }
}

const closeFlagSelector = () => {
  showFlagSelector.value = false
}

const getFlagDescription = (flag: { flag: string; value: string; descriptionKey: string }) => {
  return t(`mount.config.flagDescriptions.${flag.descriptionKey}`)
}

const handleKeydown = (event: KeyboardEvent) => {
  const key = event.key
  const ctrl = event.ctrlKey

  if (ctrl && key === 'n') {
    event.preventDefault()
    addNewConfig()
  } else if (ctrl && key === 'r') {
    event.preventDefault()
    appStore.loadRemoteConfigs()
    appStore.loadMountInfos()
  } else if (key === 'Escape') {
    event.preventDefault()
    if (showAddForm.value) {
      cancelForm()
    }
  }
}

const openInFileExplorer = async (path?: string) => {
  if (!path) {
    console.warn('Mount point path is not available')
    return
  }
  const normalizedPath = path.trim()
  try {
    await appStore.openFolder(normalizedPath)
  } catch (error: any) {
    console.error('Failed to open mount point in file explorer:', error)
    const errorMessage = error.message || error.toString() || 'Unknown error'
    if (errorMessage.includes('does not exist')) {
      console.warn(`Mount point path does not exist: ${normalizedPath}`)
    } else {
      console.error(`Failed to open file explorer: ${errorMessage}`)
    }
  }
}

const showWebdavTip = ref(!localStorage.getItem('webdav_tip_dismissed'))

const dismissWebdavTip = () => {
  showWebdavTip.value = false
  localStorage.setItem('webdav_tip_dismissed', 'true')
}

const isWindows = computed(() => {
  return typeof OS_PLATFORM !== 'undefined' && OS_PLATFORM === 'win32'
})
const showWinfspTip = ref(isWindows.value && !localStorage.getItem('winfsp_tip_dismissed'))

const dismissWinfspTip = () => {
  showWinfspTip.value = false
  localStorage.setItem('winfsp_tip_dismissed', 'true')
}

const isLinux = computed(() => {
  return typeof OS_PLATFORM !== 'undefined' && OS_PLATFORM === 'linux'
})
const showRcloneTip = ref(false)

const dismissRcloneTip = () => {
  showRcloneTip.value = false
  localStorage.setItem('rclone_tip_dismissed', 'true')
}

const shouldShowWebdavTip = computed(() => {
  if (isWindows.value) {
    return !showWinfspTip.value && showWebdavTip.value
  }
  if (isLinux.value && showRcloneTip.value) {
    return false
  }
  return showWebdavTip.value
})

onMounted(async () => {
  document.addEventListener('keydown', handleKeydown)
  rcloneStore.checkRcloneBackendStatus()
  appStore.loadRemoteConfigs()
  appStore.loadMountInfos()
  mountRefreshInterval = setInterval(appStore.loadMountInfos, 15 * 1000)
  backendStatusCheckInterval = setInterval(() => {
    rcloneStore.checkRcloneBackendStatus()
  }, 15 * 1000)
  rcloneStore.init()

  // Check rclone availability on Linux
  if (isLinux.value && !localStorage.getItem('rclone_tip_dismissed')) {
    const available = await rcloneStore.checkRcloneAvailable()
    showRcloneTip.value = !available
  }
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
  if (mountRefreshInterval) {
    clearInterval(mountRefreshInterval)
  }
  if (backendStatusCheckInterval) {
    clearInterval(backendStatusCheckInterval)
  }
})
</script>

<template>
  <div class="mount-view">
    <!-- Header Section -->
    <div class="mount-header">
      <div class="header-content">
        <div class="header-info">
          <div class="title-section">
            <HardDrive class="header-icon" />
            <h1 class="page-title">{{ t('mount.title') }}</h1>
          </div>
          <div class="stats-overview">
            <div class="stat-item">
              <span class="stat-value">{{ configCounts.total }}</span>
              <span class="stat-label">{{ t('mount.stats.total') }}</span>
            </div>
            <div class="stat-divider"></div>
            <div class="stat-item success">
              <span class="stat-value">{{ configCounts.mounted }}</span>
              <span class="stat-label">{{ t('mount.stats.mounted') }}</span>
            </div>
            <div class="stat-divider"></div>
            <div class="stat-item neutral">
              <span class="stat-value">{{ configCounts.unmounted }}</span>
              <span class="stat-label">{{ t('mount.stats.unmounted') }}</span>
            </div>
            <div v-if="configCounts.error > 0" class="stat-divider"></div>
            <div v-if="configCounts.error > 0" class="stat-item error">
              <span class="stat-value">{{ configCounts.error }}</span>
              <span class="stat-label">{{ t('mount.stats.error') }}</span>
            </div>
          </div>
        </div>

        <div class="header-actions">
          <div class="service-indicator" :class="{ active: rcloneStore.serviceRunning }">
            <div class="indicator-dot"></div>
            <span class="indicator-text">
              {{ rcloneStore.serviceRunning ? t('mount.service.running') : t('mount.service.stopped') }}
            </span>
            <button
              :class="['service-toggle', { active: rcloneStore.serviceRunning }]"
              :disabled="rcloneStore.loading"
              @click="rcloneStore.serviceRunning ? stopBackend() : startBackend()"
            >
              <component :is="rcloneStore.serviceRunning ? Square : Play" class="btn-icon" />
            </button>
          </div>
          <button class="primary-btn" @click="addNewConfig">
            <Plus class="btn-icon" />
            <span>{{ t('mount.actions.addRemote') }}</span>
          </button>
        </div>
      </div>
    </div>

    <div v-if="shouldShowWebdavTip" class="webdav-tip">
      <div class="tip-content">
        <div class="tip-icon">
          <Settings class="icon" />
        </div>
        <div class="tip-message">
          <h4 class="tip-title">{{ t('mount.tip.webdavTitle') }}</h4>
          <p class="tip-description">{{ t('mount.tip.webdavMessage') }}</p>
        </div>
        <button class="tip-close" :title="t('mount.tip.dismissForever')" @click="dismissWebdavTip">
          <X class="close-icon" />
        </button>
      </div>
    </div>

    <div v-if="showWinfspTip" class="winfsp-tip">
      <div class="tip-content">
        <div class="tip-icon">
          <HardDrive class="icon" />
        </div>
        <div class="tip-message">
          <h4 class="tip-title">{{ t('mount.tip.winfspTitle') }}</h4>
          <p class="tip-description">{{ t('mount.tip.winfspMessage') }}</p>
        </div>
        <button class="tip-close" :title="t('mount.tip.dismissForever')" @click="dismissWinfspTip">
          <X class="close-icon" />
        </button>
      </div>
    </div>

    <div v-if="showRcloneTip" class="rclone-tip">
      <div class="tip-content">
        <div class="tip-icon">
          <HardDrive class="icon" />
        </div>
        <div class="tip-message">
          <h4 class="tip-title">{{ t('mount.tip.rcloneTitle') }}</h4>
          <p class="tip-description">{{ t('mount.tip.rcloneMessage') }}</p>
        </div>
        <button class="tip-close" :title="t('mount.tip.dismissForever')" @click="dismissRcloneTip">
          <X class="close-icon" />
        </button>
      </div>
    </div>

    <!-- Controls Section -->
    <div class="controls-section">
      <div class="search-container">
        <Search class="search-icon" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('mount.filters.searchPlaceholder')"
          class="search-input"
        />
      </div>
      <div class="filter-controls">
        <select v-model="statusFilter" class="status-filter">
          <option value="all">{{ t('mount.filters.allStatus') }}</option>
          <option value="mounted">{{ t('mount.status.mounted') }}</option>
          <option value="unmounted">{{ t('mount.status.unmounted') }}</option>
          <option value="error">{{ t('mount.status.error') }}</option>
        </select>
        <button class="refresh-btn" :disabled="rcloneStore.loading" @click="appStore.loadMountInfos">
          <RefreshCw class="refresh-icon" :class="{ spinning: rcloneStore.loading }" />
        </button>
      </div>
    </div>
    <!-- Error Display -->
    <div v-if="rcloneStore.error" class="error-alert">
      <XCircle class="alert-icon" />
      <span class="alert-message">{{ rcloneStore.error }}</span>
      <button class="alert-close" @click="rcloneStore.clearError">
        <X class="close-icon" />
      </button>
    </div>

    <!-- Remote Configurations -->
    <div class="configs-container">
      <div v-if="filteredConfigs.length === 0" class="empty-state">
        <div class="empty-content">
          <Cloud class="empty-icon" />
          <h3 class="empty-title">{{ t('mount.empty.title') }}</h3>
          <p class="empty-description">{{ t('mount.empty.description') }}</p>
          <button class="empty-action-btn" @click="addNewConfig">
            <Plus class="btn-icon" />
            <span>{{ t('mount.actions.addRemote') }}</span>
          </button>
        </div>
      </div>

      <div v-else class="config-grid">
        <div
          v-for="config in filteredConfigs"
          :key="config.name"
          class="config-card"
          :class="{
            mounted: isConfigMounted(config),
            error: getConfigStatus(config) === 'error',
            loading: isConfigMounting(config)
          }"
        >
          <div class="card-header">
            <div class="config-info">
              <div class="config-icon">
                <Cloud />
              </div>
              <div class="config-details">
                <h3 class="config-name">{{ config.name }}</h3>
                <p class="config-url">{{ config.url }}</p>
              </div>
            </div>
            <div class="config-status">
              <component
                :is="getStatusIcon(getConfigStatus(config))"
                class="status-icon"
                :class="{
                  spinning: isConfigMounting(config) || appStore.loading,
                  success: getConfigStatus(config) === 'mounted',
                  error: getConfigStatus(config) === 'error'
                }"
              />
            </div>
          </div>

          <div class="card-meta">
            <div class="meta-tags">
              <span class="meta-tag">{{ config.type }}</span>
              <span
                v-if="config.mountPoint"
                class="meta-tag clickable-mount-point"
                :title="t('mount.meta.openInExplorer')"
                @click="openInFileExplorer(config.mountPoint)"
              >
                <FolderOpen class="mount-point-icon" />
                {{ config.mountPoint }}
              </span>
              <span v-if="config.volumeName" class="meta-tag">{{ config.volumeName }}</span>
              <span v-if="config.autoMount" class="meta-tag auto">{{ t('mount.meta.autoMount') }}</span>
            </div>
          </div>

          <div class="card-actions">
            <div class="action-group">
              <button
                v-if="!isConfigMounted(config)"
                class="action-btn primary"
                :disabled="isConfigMounting(config) || !config.mountPoint"
                :title="!config.mountPoint ? t('mount.messages.mountPointRequired') : ''"
                @click="mountConfig(config)"
              >
                <Play class="btn-icon" />
                <span>{{ t('mount.actions.mount') }}</span>
              </button>
              <button
                v-else
                class="action-btn warning"
                :disabled="isConfigMounting(config)"
                @click="unmountConfig(config)"
              >
                <Square class="btn-icon" />
                <span>{{ t('mount.actions.unmount') }}</span>
              </button>
            </div>

            <div class="secondary-actions">
              <button class="secondary-btn" :title="t('mount.actions.edit')" @click="editConfig(config)">
                <Edit class="btn-icon" />
              </button>
              <button
                class="secondary-btn danger"
                :disabled="isConfigMounted(config)"
                :title="t('mount.actions.delete')"
                @click="deleteConfig(config)"
              >
                <Trash2 class="btn-icon" />
              </button>
              <button
                v-if="isConfigMounted(config)"
                class="secondary-btn"
                :title="t('mount.actions.openInExplorer')"
                @click="openInFileExplorer(config.mountPoint)"
              >
                <FolderOpen class="btn-icon" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!-- Configuration Modal -->
    <div v-if="showAddForm" class="modal-backdrop">
      <div class="config-modal" @click.stop>
        <div class="modal-header">
          <div class="modal-title-section">
            <Settings class="modal-icon" />
            <h2 class="modal-title">
              {{ editingConfig ? t('mount.config.editTitle') : t('mount.config.addTitle') }}
            </h2>
          </div>
          <button class="modal-close" @click="cancelForm">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-content">
          <div class="config-form">
            <div class="form-section">
              <h3 class="section-title">{{ t('mount.config.basicInfo') }}</h3>
              <div class="form-grid">
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.name') }} *</label>
                  <input
                    v-model="configForm.name"
                    type="text"
                    class="field-input"
                    :placeholder="t('mount.config.namePlaceholder')"
                    required
                  />
                </div>
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.type') }} *</label>
                  <select v-model="configForm.type" class="field-select" required>
                    <option value="webdav">{{ t('mount.config.types.webdav') }}</option>
                  </select>
                </div>
              </div>

              <div class="form-field">
                <label class="field-label">{{ t('mount.config.url') }} *</label>
                <input
                  v-model="configForm.url"
                  type="url"
                  class="field-input"
                  :placeholder="t('mount.config.urlPlaceholder')"
                  required
                />
              </div>

              <div v-if="configForm.type === 'webdav'" class="form-field">
                <label class="field-label">{{ t('mount.config.vendor') }}</label>
                <input
                  v-model="configForm.vendor"
                  type="text"
                  class="field-input"
                  :placeholder="t('mount.config.vendorPlaceholder')"
                />
              </div>
            </div>

            <div class="form-section">
              <h3 class="section-title">{{ t('mount.config.authentication') }}</h3>
              <div class="form-grid">
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.username') }} *</label>
                  <input
                    v-model="configForm.user"
                    type="text"
                    class="field-input"
                    :placeholder="t('mount.config.usernamePlaceholder')"
                    required
                  />
                </div>
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.password') }} *</label>
                  <input
                    v-model="configForm.pass"
                    type="text"
                    class="field-input"
                    :placeholder="t('mount.config.passwordPlaceholder')"
                    required
                  />
                </div>
              </div>
            </div>

            <div class="form-section">
              <h3 class="section-title">{{ t('mount.config.mountSettings') }}</h3>
              <div class="form-grid">
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.mountPoint') }}</label>
                  <input
                    v-model="configForm.mountPoint"
                    type="text"
                    class="field-input"
                    :placeholder="t('mount.config.mountPointPlaceholder')"
                  />
                </div>
                <div class="form-field">
                  <label class="field-label">{{ t('mount.config.volumeName') }}</label>
                  <input
                    v-model="configForm.volumeName"
                    type="text"
                    class="field-input"
                    :placeholder="t('mount.config.volumeNamePlaceholder')"
                  />
                </div>
              </div>

              <div class="form-field">
                <label class="checkbox-field">
                  <input v-model="configForm.autoMount" type="checkbox" class="checkbox-input" />
                  <span class="checkbox-label">{{ t('mount.config.autoMount') }}</span>
                </label>
              </div>
            </div>

            <div class="form-section">
              <h3 class="section-title">{{ t('mount.config.advancedSettings') }}</h3>
              <div class="form-field">
                <label class="field-label">{{ t('mount.config.extraFlags') }}</label>

                <div class="flags-header">
                  <button
                    type="button"
                    class="quick-flags-btn"
                    :title="t('mount.config.quickFlagsTooltip')"
                    @click="showFlagSelector = !showFlagSelector"
                  >
                    <Settings class="btn-icon" />
                    <span>{{ t('mount.config.quickFlags') }}</span>
                  </button>
                </div>

                <div v-if="showFlagSelector" class="flag-selector-backdrop" @click="closeFlagSelector">
                  <div class="flag-selector-popup" @click.stop>
                    <div class="flag-selector-header">
                      <h4>{{ t('mount.config.selectCommonFlags') }}</h4>
                      <button class="close-selector-btn" @click="closeFlagSelector">
                        <X class="btn-icon" />
                      </button>
                    </div>

                    <div class="flag-selector-content">
                      <div class="flag-selector-help">
                        <p>{{ t('mount.config.clickToToggleFlags') }}</p>
                      </div>

                      <div class="flag-categories">
                        <div v-for="category in commonFlags" :key="category.category" class="flag-category">
                          <div class="category-header">
                            <h5>{{ t(`mount.config.flagCategories.${category.category}`) }}</h5>
                          </div>
                          <div class="category-flags">
                            <div
                              v-for="flag in category.flags"
                              :key="`${flag.flag}-${flag.value}`"
                              class="flag-option"
                              :class="{
                                selected: isFlagInConfig(flag),
                                'in-config': isFlagInConfig(flag)
                              }"
                              :title="getFlagDescription(flag)"
                              @click="toggleFlag(flag)"
                            >
                              <div class="flag-checkbox">
                                <div class="custom-checkbox" :class="{ checked: isFlagInConfig(flag) }">
                                  <CheckCircle v-if="isFlagInConfig(flag)" class="check-icon" />
                                </div>
                              </div>
                              <div class="flag-content">
                                <code class="flag-code">{{ flag.flag }}{{ flag.value ? `=${flag.value}` : '' }}</code>
                                <span class="flag-description">{{ getFlagDescription(flag) }}</span>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- Manual Flags Input -->
                <div class="flags-container">
                  <div v-for="(_, index) in configForm.extraFlags || []" :key="index" class="flag-item">
                    <input
                      v-model="configForm.extraFlags![index]"
                      type="text"
                      class="flag-input"
                      :placeholder="t('mount.config.flagPlaceholder')"
                    />
                    <button
                      type="button"
                      class="remove-flag-btn"
                      :title="t('mount.config.removeFlag')"
                      @click="removeFlag(index)"
                    >
                      <X class="btn-icon" />
                    </button>
                  </div>
                  <button type="button" class="add-flag-btn" @click="addFlag">
                    <Plus class="btn-icon" />
                    <span>{{ t('mount.config.addFlag') }}</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="cancel-btn" @click="cancelForm">
            <X class="btn-icon" />
            <span>{{ t('common.cancel') }}</span>
          </button>
          <button class="save-btn" :disabled="appStore.loading" @click="saveConfig">
            <Save class="btn-icon" />
            <span>{{ editingConfig ? t('common.save') : t('common.add') }}</span>
          </button>
        </div>
      </div>
    </div>
    <ConfirmDialog
      :is-open="showConfirmDialog"
      :title="confirmDialogConfig.title"
      :message="confirmDialogConfig.message"
      :confirm-text="t('mount.actions.delete')"
      :cancel-text="t('common.cancel')"
      variant="danger"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </div>
</template>

<style scoped src="./css/MountView.css"></style>
