<script setup lang="ts">
import * as chrono from 'chrono-node'
import {
  AlertCircle,
  AlertTriangle,
  ArrowDown,
  ArrowUp,
  Copy,
  Download,
  Filter,
  FolderOpen,
  Info,
  Maximize2,
  Minimize2,
  Pause,
  Play,
  RotateCcw,
  Search,
  Settings,
  Trash2
} from 'lucide-vue-next'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import ConfirmDialog from '../components/ui/ConfirmDialog.vue'
import { useTranslation } from '../composables/useI18n'
import { useAppStore } from '../stores/app'

type filterSourceType = 'openlist' | 'rclone' | 'app' | 'service' | 'all'

const appStore = useAppStore()
const { t } = useTranslation()
const logContainer = ref<HTMLElement>()
const searchInputRef = ref<HTMLInputElement>()
const autoScroll = ref(true)
const isPaused = ref(false)
const filterLevel = ref<string>(appStore.settings.app.log_filter_level || 'all')
const filterSource = ref<string>(appStore.settings.app.log_filter_source || 'openlist')
const searchQuery = ref('')
const selectedEntries = ref<Set<number>>(new Set())
const showFilters = ref(true)
const showSettings = ref(false)
const fontSize = ref(13)
const maxLines = ref(1000)
const isCompactMode = ref(false)
const isFullscreen = ref(false)
const stripAnsiColors = ref(true)
const showNotification = ref(false)
const notificationMessage = ref('')
const notificationType = ref<'success' | 'info' | 'warning' | 'error'>('success')
const showConfirmDialog = ref(false)
const confirmDialogConfig = ref({
  title: '',
  message: '',
  onConfirm: () => {},
  onCancel: () => {}
})

watch(filterLevel, async newValue => {
  appStore.settings.app.log_filter_level = newValue
  await appStore.saveSettings()
})

watch(filterSource, async newValue => {
  appStore.settings.app.log_filter_source = newValue
  await appStore.saveSettings()
  await appStore.loadLogs((newValue !== 'gin' ? newValue : 'openlist') as filterSourceType)
  await scrollToBottom()
})

let logRefreshInterval: NodeJS.Timeout | null = null

const showNotificationMessage = (message: string, type: 'success' | 'info' | 'warning' | 'error' = 'success') => {
  notificationMessage.value = message
  notificationType.value = type
  showNotification.value = true

  setTimeout(() => {
    showNotification.value = false
  }, 3000)
}

const openLogsDirectory = async () => {
  try {
    await appStore.openLogsDirectory()
    showNotificationMessage(t('logs.notifications.openDirectorySuccess'), 'success')
  } catch (error) {
    console.error('Failed to open logs directory:', error)
    showNotificationMessage(t('logs.notifications.openDirectoryFailed'), 'error')
  }
}

const stripAnsiCodes = (text: string): string => {
  return text.replace(/\u001b\[[0-9;]*[mGKHF]/g, '')
}

const parseLogEntry = (logText: string) => {
  const cleanText = stripAnsiColors.value ? stripAnsiCodes(logText).trim() : logText.trim()
  const originalText = logText.trim()

  let level = 'info'
  let timestamp = ''
  let source = 'openlist'
  let message = cleanText

  const levelMatch = cleanText.match(/^(WARN|ERROR|INFO|DEBUG|info|debug|warn|error)/i)
  if (levelMatch) {
    level = levelMatch[1].toLowerCase()
  }

  const timestampMatch = cleanText.match(/(\d{4}[-/]\d{2}[-/]\d{2}[T\s-]*\d{2}:\d{2}:\d{2})/)
  if (timestampMatch) {
    timestamp = timestampMatch[1]
  } else {
    timestamp = chrono.parseDate(cleanText)?.toISOString().replace('T', ' ').substring(0, 19) || ''
  }

  if (cleanText.includes('[GIN]')) {
    source = 'gin'
    level = 'info'
    const ginMatch = cleanText.match(/\[GIN\]\s*(.+)/)
    if (ginMatch) {
      message = ginMatch[1]
      const statusMatch = message.match(/\|\s*(\d{3})\s*\|/)
      if (statusMatch) {
        const statusCode = parseInt(statusMatch[1])
        if (statusCode >= 400 && statusCode < 500) {
          level = 'warn'
        } else if (statusCode >= 500) {
          level = 'error'
        }
      }
    }
  } else {
    source = filterSource.value
  }

  message = message
    .replace(/^(WARN|ERROR|INFO|DEBUG)\s*/i, '')
    .replace(/^\[\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}\]\s*/, '')
    .replace(/^\d{4}\/\d{2}\/\d{2}\s*-\s*\d{2}:\d{2}:\d{2}\s*\|\s*/, '')
    .trim()

  const timeMatch = timestamp.match(/(\d{2}:\d{2}:\d{2})/)
  const displayTime = timeMatch ? timeMatch[1] : timestamp

  return {
    timestamp: displayTime,
    level,
    source,
    message: message || cleanText,
    original: cleanText,
    rawMessage: stripAnsiColors.value ? message : originalText,
    fullTimestamp: timestamp
  }
}

const filteredLogs = computed(() => {
  let logs = appStore.logs
    .slice(-maxLines.value)
    .filter((log: string | string[]) => !log.includes('/ping'))
    .map(parseLogEntry)

  if (filterLevel.value !== 'all') {
    logs = logs.filter((log: any) => log.level === filterLevel.value)
  }

  if (filterSource.value !== 'all') {
    logs = logs.filter((log: any) => log.source === filterSource.value)
  }

  if (searchQuery.value.trim()) {
    const query = searchQuery.value.toLowerCase()
    logs = logs.filter(
      (log: any) =>
        log.message.toLowerCase().includes(query) ||
        log.source.toLowerCase().includes(query) ||
        log.level.toLowerCase().includes(query)
    )
  }

  return logs
})

const logLevelClass = (level: string) => {
  switch (level) {
    case 'error':
      return 'log-error'
    case 'warn':
      return 'log-warning'
    case 'info':
      return 'log-info'
    case 'debug':
      return 'log-debug'
    default:
      return 'log-info'
  }
}

const scrollToBottom = async () => {
  if (autoScroll.value && !isPaused.value && logContainer.value) {
    await nextTick()
    logContainer.value.scrollTop = logContainer.value.scrollHeight
  }
}

const scrollToTop = () => {
  if (logContainer.value) {
    logContainer.value.scrollTop = 0
  }
}

const clearLogs = async () => {
  confirmDialogConfig.value = {
    title: t('logs.messages.confirmTitle') || t('common.confirm'),
    message: t('logs.messages.confirmClear'),
    onConfirm: async () => {
      showConfirmDialog.value = false
      try {
        await appStore.clearLogs(
          (filterSource.value !== 'all' && filterSource.value !== 'gin'
            ? filterSource.value
            : 'openlist') as filterSourceType
        )
        selectedEntries.value.clear()
        showNotificationMessage(t('logs.notifications.clearSuccess'), 'success')
      } catch (error) {
        console.error('Failed to clear logs:', error)
        showNotificationMessage(t('logs.notifications.clearFailed'), 'error')
      }
    },
    onCancel: () => {
      showConfirmDialog.value = false
    }
  }

  showConfirmDialog.value = true
}

const copyLogsToClipboard = async () => {
  let logsToExport = filteredLogs.value

  if (selectedEntries.value.size > 0) {
    logsToExport = filteredLogs.value.filter((_, index) => selectedEntries.value.has(index))
  }

  const logsText = logsToExport
    .map((log: any) => `[${log.timestamp || 'N/A'}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
    .join('\n')

  try {
    await navigator.clipboard.writeText(logsText)
    const count = selectedEntries.value.size > 0 ? selectedEntries.value.size : filteredLogs.value.length
    showNotificationMessage(t('logs.notifications.copySuccess', { count }), 'success')
  } catch (error) {
    console.error('Failed to copy logs:', error)
    showNotificationMessage(t('logs.notifications.copyFailed'), 'error')
  }
}

const exportLogs = () => {
  let logsToExport = filteredLogs.value

  if (selectedEntries.value.size > 0) {
    logsToExport = filteredLogs.value.filter((_, index) => selectedEntries.value.has(index))
  }

  const logsText = logsToExport
    .map((log: any) => `[${log.timestamp || 'N/A'}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
    .join('\n')

  const blob = new Blob([logsText], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `openlist-logs-${new Date().toISOString().split('T')[0]}.txt`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)

  const count = selectedEntries.value.size > 0 ? selectedEntries.value.size : filteredLogs.value.length
  showNotificationMessage(t('logs.notifications.exportSuccess', { count }), 'success')
}

const toggleSelectEntry = (index: number) => {
  if (selectedEntries.value.has(index)) {
    selectedEntries.value.delete(index)
  } else {
    selectedEntries.value.add(index)
  }
}

const selectAllVisible = () => {
  filteredLogs.value.forEach((_: any, index: number) => {
    selectedEntries.value.add(index)
  })
}

const clearSelection = () => {
  selectedEntries.value.clear()
}

const togglePause = () => {
  isPaused.value = !isPaused.value
}

const refreshLogs = async () => {
  await appStore.loadLogs(
    (filterSource.value !== 'all' && filterSource.value !== 'gin' ? filterSource.value : 'openlist') as filterSourceType
  )
  await scrollToBottom()
  if (isPaused.value) {
    isPaused.value = false
  }
}

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
  if (isFullscreen.value) {
    document.documentElement.requestFullscreen?.()
  } else {
    document.exitFullscreen?.()
  }
}

const handleKeydown = (event: KeyboardEvent) => {
  const ctrl = event.ctrlKey
  const key = event.key.toLowerCase()

  if (ctrl) {
    switch (key) {
      case 'f':
        event.preventDefault()
        searchInputRef.value?.focus()
        break
      case 'a':
        event.preventDefault()
        selectAllVisible()
        break
      case 'c':
        if (selectedEntries.value.size > 0) {
          event.preventDefault()
          copyLogsToClipboard()
        }
        break
      case 'r':
        event.preventDefault()
        refreshLogs()
        break
      case 'delete':
        event.preventDefault()
        clearLogs()
        break
    }
  } else {
    switch (key) {
      case 'escape':
        clearSelection()
        searchQuery.value = ''
        break
      case 'home':
        event.preventDefault()
        scrollToTop()
        break
      case 'end':
        event.preventDefault()
        scrollToBottom()
        break
      case 'f11':
        event.preventDefault()
        toggleFullscreen()
        break
      case ' ':
        if (event.target === document.body) {
          event.preventDefault()
          togglePause()
        }
        break
    }
  }
}

onMounted(async () => {
  appStore.loadLogs((filterSource.value !== 'gin' ? filterSource.value : 'openlist') as filterSourceType).then(() => {
    scrollToBottom()
  })

  document.addEventListener('keydown', handleKeydown)

  logRefreshInterval = setInterval(async () => {
    if (!isPaused.value) {
      const oldLength = appStore.logs.length
      await appStore.loadLogs((filterSource.value !== 'gin' ? filterSource.value : 'openlist') as filterSourceType)

      if (appStore.logs.length > oldLength) {
        await scrollToBottom()
      }
    }
  }, 30 * 1000)
})

onUnmounted(() => {
  if (logRefreshInterval) {
    clearInterval(logRefreshInterval)
  }
  document.removeEventListener('keydown', handleKeydown)
})

const unwatchLogs = appStore.$subscribe(mutation => {
  if (mutation.storeId === 'app') {
    const events = Array.isArray(mutation.events) ? mutation.events : [mutation.events]
    if (events.some((event: any) => event.key === 'logs')) {
      scrollToBottom()
    }
  }
})

onUnmounted(() => {
  unwatchLogs()
})
</script>

<template>
  <div class="log-view" :class="{ fullscreen: isFullscreen, compact: isCompactMode }">
    <div class="log-toolbar">
      <div class="toolbar-section left">
        <button
          class="toolbar-btn"
          :class="{ active: isPaused }"
          :title="isPaused ? t('logs.toolbar.resume') : t('logs.toolbar.pause')"
          @click="togglePause"
        >
          <Pause v-if="!isPaused" :size="16" />
          <Play v-else :size="16" />
        </button>

        <button class="toolbar-btn" :title="t('logs.toolbar.refresh')" @click="refreshLogs">
          <RotateCcw :size="16" />
        </button>

        <div class="toolbar-separator"></div>

        <button
          class="toolbar-btn"
          :class="{ active: showFilters }"
          :title="t('logs.toolbar.showFilters')"
          @click="showFilters = !showFilters"
        >
          <Filter :size="16" />
        </button>

        <button
          class="toolbar-btn"
          :class="{ active: showSettings }"
          :title="t('logs.toolbar.settings')"
          @click="showSettings = !showSettings"
        >
          <Settings :size="16" />
        </button>
      </div>

      <div class="toolbar-section center">
        <div class="search-container">
          <Search :size="14" class="search-icon" />
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            :placeholder="t('logs.search.placeholder')"
            class="search-input"
            @keydown.escape="searchQuery = ''"
          />
        </div>
      </div>

      <div class="toolbar-section right">
        <div class="log-stats">
          <span class="stat">{{
            t('logs.stats.logsCount', { filtered: filteredLogs.length, total: appStore.logs.length })
          }}</span>
          <span v-if="selectedEntries.size > 0" class="stat selected">
            {{ t('logs.stats.selected', { count: selectedEntries.size }) }}
          </span>
        </div>

        <div class="toolbar-separator"></div>

        <button
          class="toolbar-btn"
          :title="t('logs.toolbar.copyToClipboard')"
          :disabled="filteredLogs.length === 0"
          @click="copyLogsToClipboard"
        >
          <Copy :size="16" />
        </button>

        <button
          class="toolbar-btn"
          :title="t('logs.toolbar.exportLogs')"
          :disabled="filteredLogs.length === 0"
          @click="exportLogs"
        >
          <Download :size="16" />
        </button>

        <button
          class="toolbar-btn danger"
          :disabled="filteredLogs.length === 0 || filterSource === 'gin' || filterSource === 'all'"
          :title="t('logs.toolbar.clearLogs')"
          @click="clearLogs"
        >
          <Trash2 :size="16" />
        </button>

        <button class="toolbar-btn" :title="t('logs.toolbar.openLogsDirectory')" @click="openLogsDirectory">
          <FolderOpen :size="16" />
        </button>

        <div class="toolbar-separator"></div>

        <button class="toolbar-btn" :title="t('logs.toolbar.toggleFullscreen')" @click="toggleFullscreen">
          <Maximize2 v-if="!isFullscreen" :size="16" />
          <Minimize2 v-else :size="16" />
        </button>
      </div>
    </div>

    <div v-if="showFilters" class="filters-panel">
      <div class="filter-group">
        <label>{{ t('logs.filters.labels.level') }}</label>
        <select v-model="filterLevel" class="filter-select">
          <option value="all">{{ t('logs.filters.levels.all') }}</option>
          <option value="debug">{{ t('logs.filters.levels.debug') }}</option>
          <option value="info">{{ t('logs.filters.levels.info') }}</option>
          <option value="warn">{{ t('logs.filters.levels.warn') }}</option>
          <option value="error">{{ t('logs.filters.levels.error') }}</option>
        </select>
      </div>
      <div class="filter-group">
        <label>{{ t('logs.filters.labels.source') }}</label>
        <select v-model="filterSource" class="filter-select">
          <option value="all">{{ t('logs.filters.sources.all') }}</option>
          <option value="openlist">{{ t('logs.filters.sources.openlist') }}</option>
          <option value="gin">GIN Server</option>
          <option value="rclone">{{ t('logs.filters.sources.rclone') }}</option>
          <option value="service">{{ t('logs.filters.sources.service') }}</option>
          <option value="app">{{ t('logs.filters.app') }}</option>
        </select>
      </div>

      <div class="filter-actions">
        <button class="filter-btn" :disabled="filteredLogs.length === 0" @click="selectAllVisible">
          {{ t('logs.filters.actions.selectAll') }}
        </button>

        <button class="filter-btn" :disabled="selectedEntries.size === 0" @click="clearSelection">
          {{ t('logs.filters.actions.clearSelection') }}
        </button>

        <label class="checkbox-label">
          <input v-model="autoScroll" type="checkbox" class="checkbox" />
          {{ t('logs.filters.actions.autoScroll') }}
        </label>
      </div>
    </div>

    <div v-if="showSettings" class="settings-panel">
      <div class="setting-group">
        <label>{{ t('logs.settings.fontSize') }}</label>
        <input v-model="fontSize" type="range" min="10" max="20" class="range-input" />
        <span class="setting-value">{{ fontSize }}px</span>
      </div>

      <div class="setting-group">
        <label>{{ t('logs.settings.maxLines') }}</label>
        <input v-model="maxLines" type="number" min="100" max="10000" step="100" class="number-input" />
      </div>
      <div class="setting-group">
        <label class="checkbox-label">
          <input v-model="isCompactMode" type="checkbox" class="checkbox" />
          {{ t('logs.settings.compactMode') }}
        </label>

        <label class="checkbox-label">
          <input v-model="stripAnsiColors" type="checkbox" class="checkbox" />
          {{ t('logs.settings.stripAnsiColors') }}
        </label>
      </div>
    </div>
    <div class="log-container">
      <div class="log-header">
        <div class="log-col timestamp">{{ t('logs.headers.timestamp') }}</div>
        <div class="log-col level">{{ t('logs.headers.level') }}</div>
        <div class="log-col source">{{ t('logs.headers.source') }}</div>
        <div class="log-col message">{{ t('logs.headers.message') }}</div>
        <div class="log-col actions">
          <button class="scroll-btn" :title="t('logs.toolbar.scrollToTop')" @click="scrollToTop">
            <ArrowUp :size="14" />
          </button>
          <button class="scroll-btn" :title="t('logs.toolbar.scrollToBottom')" @click="scrollToBottom">
            <ArrowDown :size="14" />
          </button>
        </div>
      </div>

      <div ref="logContainer" class="log-content" :style="{ fontSize: fontSize + 'px' }">
        <div v-if="filteredLogs.length === 0" class="empty-state">
          <div class="empty-icon">📄</div>
          <h3>{{ t('logs.viewer.noLogsFound') }}</h3>
          <p v-if="searchQuery">{{ t('logs.viewer.noLogsMatch') }}</p>
          <p v-else>{{ t('logs.viewer.logsWillAppear') }}</p>
        </div>
        <div
          v-for="(log, index) in filteredLogs"
          :key="index"
          class="log-entry"
          :class="[
            logLevelClass(log.level),
            {
              selected: selectedEntries.has(index),
              compact: isCompactMode
            }
          ]"
          @click="toggleSelectEntry(index)"
        >
          <div class="log-col timestamp">
            {{ log.timestamp || '--:--:--' }}
          </div>
          <div class="log-col level">
            <span class="level-badge" :class="log.level">
              {{ log.level.toUpperCase() }}
            </span>
          </div>
          <div class="log-col source" :data-source="log.source">
            {{ log.source }}
          </div>
          <div class="log-col message">
            {{ log.message }}
          </div>
        </div>
      </div>
    </div>

    <div class="status-bar">
      <div class="status-left">
        <span class="status-item">
          {{ t('logs.status.autoScroll') }} {{ autoScroll ? t('logs.status.on') : t('logs.status.off') }}
        </span>
        <span class="status-item">
          {{ t('logs.status.updates') }} {{ isPaused ? t('logs.status.paused') : t('logs.status.live') }}
        </span>
      </div>

      <div class="status-right">
        <span class="status-item">
          {{ t('logs.status.showing', { filtered: filteredLogs.length, total: appStore.logs.length }) }}
        </span>
      </div>
    </div>

    <Transition name="notification">
      <div v-if="showNotification" class="notification-toast" :class="[`notification-${notificationType}`]">
        <div class="notification-content">
          <div class="notification-icon">
            <Copy v-if="notificationType === 'success'" :size="20" />
            <AlertCircle v-else-if="notificationType === 'error'" :size="20" />
            <Info v-else-if="notificationType === 'info'" :size="20" />
            <AlertTriangle v-else-if="notificationType === 'warning'" :size="20" />
          </div>
          <span class="notification-message">{{ notificationMessage }}</span>
        </div>
      </div>
    </Transition>

    <ConfirmDialog
      :is-open="showConfirmDialog"
      :title="confirmDialogConfig.title"
      :message="confirmDialogConfig.message"
      :confirm-text="t('common.confirm')"
      :cancel-text="t('common.cancel')"
      variant="danger"
      @confirm="confirmDialogConfig.onConfirm"
      @cancel="confirmDialogConfig.onCancel"
    />
  </div>
</template>

<style scoped src="./css/LogView.css"></style>
