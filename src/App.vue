<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { useAppStore } from './stores/app'
import { useTranslation } from './composables/useI18n'
import { useTray } from './composables/useTray'
import { TauriAPI } from './api/tauri'
import Navigation from './components/Navigation.vue'
import TitleBar from './components/ui/TitleBar.vue'
import { platform } from '@tauri-apps/plugin-os'

const appStore = useAppStore()
const { t } = useTranslation()
const { updateTrayMenu } = useTray()
const router = useRouter()
const isLoading = ref(true)
const isMacOS = ref(false)

let updateUnlisten: (() => void) | null = null

const handleKeydown = (event: KeyboardEvent) => {
  const { ctrlKey, key } = event

  if (!ctrlKey) return

  switch (key.toLowerCase()) {
    case 'h':
      event.preventDefault()
      router.push('/')
      break
    case 'm':
      event.preventDefault()
      router.push('/mount')
      break
    case 'u':
      event.preventDefault()
      router.push('/update')
      break
    case 'l':
      event.preventDefault()
      router.push('/logs')
      break
    case ',':
      event.preventDefault()
      router.push('/settings')
      break
  }
}

onMounted(async () => {
  isMacOS.value = platform() === 'macos'
  try {
    appStore.init()
    appStore.applyTheme(appStore.settings.app.theme || 'light')
    await updateTrayMenu(appStore.openlistCoreStatus.running)

    try {
      updateUnlisten = await TauriAPI.updater.onBackgroundUpdate(updateInfo => {
        console.log('Global update listener: Update available', updateInfo)
        appStore.setUpdateAvailable(true, updateInfo)
      })
    } catch (err) {
      console.warn('Failed to set up global update listener:', err)
    }
    document.addEventListener('keydown', handleKeydown)
  } finally {
    isLoading.value = false
  }
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
  try {
    updateUnlisten?.()
  } catch (err) {
    console.warn('Error cleaning up global update listener:', err)
  }
})
</script>

<template>
  <div v-if="isLoading" class="loading-screen">
    <div class="loading-backdrop"></div>
    <div class="loading-content">
      <div class="loading-logo">
        <div class="logo-container">
          <div class="logo-icon">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path
                d="M3 7V17C3 18.1046 3.89543 19 5 19H19C20.1046 19 21 18.1046 21 17V7"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M21 7L12 13L3 7"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M7 3H17C18.1046 3 19 3.89543 19 5V7H5V5C5 3.89543 5.89543 3 7 3Z"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
          <div class="logo-shimmer"></div>
        </div>
      </div>
      <h1 class="loading-title">
        <span class="title-main">{{ t('app.title').split(' ')[0] }}</span>
        <span class="title-sub">{{ t('app.title').split(' ')[1] }}</span>
      </h1>
      <p class="loading-subtitle">{{ t('app.loading') }}</p>
      <div class="loading-progress">
        <div class="progress-bar">
          <div class="progress-fill"></div>
        </div>
      </div>
    </div>
  </div>
  <div v-else id="app" class="app-container" :style="{ paddingTop: isMacOS ? '0' : '32px' }">
    <!-- Custom Title Bar -->
    <TitleBar v-if="!isMacOS" />

    <div class="app-background">
      <div class="bg-gradient-primary"></div>
      <div class="bg-gradient-secondary"></div>
    </div>

    <Navigation />

    <main class="main-content">
      <div class="content-container">
        <router-view v-slot="{ Component, route }">
          <transition name="page" mode="out-in">
            <component :is="Component" :key="route.path" />
          </transition>
        </router-view>
      </div>
    </main>
  </div>
</template>

<style>
*,
*::before,
*::after {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

:root {
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  font-weight: 400;

  --color-text-primary: #1d1d1f;
  --color-text-secondary: #6e6e73;
  --color-text-tertiary: #86868b;
  --color-background-primary: #ffffff;
  --color-background-secondary: #f5f5f7;
  --color-background-tertiary: #fbfbfd;
  --color-surface: rgba(255, 255, 255, 0.8);
  --color-surface-elevated: rgba(255, 255, 255, 0.95);
  --color-border: rgba(0, 0, 0, 0.1);
  --color-border-secondary: rgba(0, 0, 0, 0.05);
  --color-primary: #6366f1;
  --color-primary-hover: #4f46e5;
  --color-accent: #007aff;
  --color-accent-hover: #0056b3;
  --color-success: #34c759;
  --color-warning: #ff9500;
  --color-danger: #ff3b30;

  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.06);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.05), 0 2px 4px rgba(0, 0, 0, 0.06);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.08), 0 4px 6px rgba(0, 0, 0, 0.05);
  --shadow-xl: 0 20px 25px rgba(0, 0, 0, 0.1), 0 10px 10px rgba(0, 0, 0, 0.04);

  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-2xl: 20px;

  --transition-fast: 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  --transition-medium: 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  --transition-slow: 0.35s cubic-bezier(0.4, 0, 0.2, 1);

  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}

:root.dark,
:root.auto.dark {
  --color-text-primary: #f5f5f7;
  --color-text-secondary: #a1a1a6;
  --color-text-tertiary: #86868b;
  --color-background-primary: #000000;
  --color-background-secondary: #1c1c1e;
  --color-background-tertiary: #2c2c2e;
  --color-surface: rgba(28, 28, 30, 0.8);
  --color-surface-elevated: rgba(44, 44, 46, 0.95);
  --color-border: rgba(255, 255, 255, 0.1);
  --color-border-secondary: rgba(255, 255, 255, 0.05);
  --color-primary: #6366f1;
  --color-primary-hover: #818cf8;
  --color-accent: #0a84ff;
  --color-accent-hover: #409cff;
}

:root.dark,
:root.auto.dark {
  .metric-card {
    background: var(--color-background-tertiary);
    border-color: var(--color-border);
    color: var(--color-text-primary);
  }

  .metric-title {
    color: var(--color-text-primary);
  }

  .metric-value {
    color: var(--color-text-primary);
  }

  .metric-description {
    color: var(--color-text-tertiary);
  }

  .metric-icon {
    color: var(--color-accent);
  }

  .service-info,
  .heartbeat-section,
  .info-item {
    color: var(--color-text-primary);
  }

  .heartbeat-header h4 {
    color: var(--color-text-primary);
  }

  .metric.healthy {
    color: var(--color-success);
  }

  .metric.warning {
    color: var(--color-warning);
  }

  .metric.error {
    color: var(--color-danger);
  }

  .metric.success {
    color: var(--color-success);
  }

  .status-indicator {
    color: var(--color-text-primary);
  }

  .status-indicator.online {
    color: var(--color-success);
  }

  .status-indicator.offline {
    color: var(--color-danger);
  }

  .nav-item {
    color: var(--color-text-secondary);
  }

  .nav-item.active {
    color: var(--color-accent);
  }

  h1,
  h2,
  h3,
  h4,
  h5,
  h6 {
    color: var(--color-text-primary);
  }

  p,
  span,
  div {
    color: inherit;
  }

  svg {
    color: inherit;
  }

  input,
  select,
  textarea {
    background: var(--color-surface);
    border-color: var(--color-border);
    color: var(--color-text-primary);
  }

  input::placeholder,
  textarea::placeholder {
    color: var(--color-text-tertiary);
  }

  .progress-bar {
    background: var(--color-background-secondary);
  }

  .progress-fill {
    background: var(--color-accent);
  }

  .progress-fill.warning {
    background: var(--color-warning);
  }

  .progress-fill.danger {
    background: var(--color-danger);
  }

  button {
    color: var(--color-text-primary);
    border-color: var(--color-border);
  }

  button:hover {
    background: var(--color-surface-elevated);
  }
}

body {
  color: var(--color-text-primary);
  background-color: var(--color-background-primary);
  font-family: inherit;
  overflow: hidden;
}

.loading-screen {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  overflow: hidden;
}

.loading-backdrop {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 25% 25%, rgba(120, 119, 198, 0.3) 0%, transparent 50%),
    radial-gradient(circle at 75% 75%, rgba(255, 255, 255, 0.1) 0%, transparent 50%);
}

.loading-content {
  text-align: center;
  z-index: 1;
}

.loading-logo {
  margin-bottom: 2rem;
  display: flex;
  justify-content: center;
}

.logo-container {
  position: relative;
  width: 80px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo-icon {
  position: relative;
  z-index: 2;
  color: white;
  filter: drop-shadow(0 8px 16px rgba(0, 0, 0, 0.2));
}

.logo-shimmer {
  position: absolute;
  inset: -20px;
  background: conic-gradient(from 0deg, transparent, rgba(255, 255, 255, 0.2), transparent);
  border-radius: 50%;
}

.loading-title {
  margin-bottom: 0.5rem;
  font-size: 2.5rem;
  font-weight: 300;
  color: white;
  letter-spacing: -0.02em;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
}

.title-main {
  font-weight: 600;
  background: linear-gradient(135deg, #ffffff 0%, #e0e0e0 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.title-sub {
  font-weight: 300;
  font-size: 0.6em;
  opacity: 0.9;
}

.loading-subtitle {
  color: rgba(255, 255, 255, 0.8);
  font-size: 0.875rem;
  margin-bottom: 2rem;
  font-weight: 400;
}

.loading-progress {
  width: 200px;
  margin: 0 auto;
}

.progress-bar {
  width: 100%;
  height: 2px;
  background-color: rgba(255, 255, 255, 0.2);
  border-radius: 1px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 1px;
}

.app-container {
  position: relative;
  height: 100vh;
  display: flex;
  overflow: hidden;
  background-color: var(--color-background-primary);
  padding-top: 32px;
}

.app-background {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
}

.bg-gradient-primary {
  position: absolute;
  top: -50%;
  right: -30%;
  width: 80%;
  height: 80%;
  background: radial-gradient(circle, rgba(0, 122, 255, 0.05) 0%, transparent 70%);
  border-radius: 50%;
}

.bg-gradient-secondary {
  position: absolute;
  bottom: -40%;
  left: -20%;
  width: 60%;
  height: 60%;
  background: radial-gradient(circle, rgba(175, 82, 222, 0.03) 0%, transparent 70%);
  border-radius: 50%;
}

.main-content {
  position: relative;
  z-index: 1;
  flex: 1;
  height: 100vh;
  overflow: scroll;
  background-color: var(--color-background-secondary);
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.main-content::-webkit-scrollbar {
  display: none;
}

.content-container {
  height: 100%;
  padding: 0.3 rem;
  max-width: none;
  margin: 0;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(24px) scale(0.98);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(1.02);
}

::-webkit-scrollbar {
  width: 12px;
  height: 12px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background-color: var(--color-border);
  border-radius: 6px;
  border: 3px solid var(--color-background-primary);
  transition: background-color var(--transition-fast);
}

::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-text-tertiary);
}

::-webkit-scrollbar-corner {
  background: var(--color-background-primary);
}

::selection {
  background-color: rgba(0, 122, 255, 0.2);
  color: var(--color-text-primary);
}

:focus {
  outline: none;
}

:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
</style>
