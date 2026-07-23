<template>
  <div class="flex flex-col gap-4 w-full justify-center p-4">
    <div class="flex gap-2 justify-start items-center">
      <Settings class="text-accent" />
      <h4 class="font-semibold text-main">{{ t('dashboard.quickActions.title') }}</h4>
    </div>
    <div class="flex flex-row w-full gap-4">
      <div class="flex flex-col flex-1 gap-2 border p-2 rounded-md border-border-secondary bg-surface shadow-sm">
        <div class="flex flex-wrap gap-2 items-center">
          <h4 class="text-main font-semibold text-sm">{{ t('dashboard.quickActions.openlistService') }}</h4>
          <div v-if="isCoreLoading" class="flex items-center">
            <div class="border-3 border-border w-4 h-4 rounded-full border-t-3 border-t-accent animate-spin"></div>
          </div>
        </div>
        <div class="flex flex-wrap gap-2 items-center w-full">
          <CustomButton v-if="isCoreLoading" class="flex-1" :icon="Loader" text="" type="secondary" disabled />
          <CustomButton
            v-else-if="isCoreRunning"
            type="custom"
            class="bg-danger/80! hover:bg-danger! flex-1"
            text-class="text-white"
            icon-class="text-white"
            :icon="Square"
            :text="t('dashboard.quickActions.stopOpenListCore')"
            @click="toggleCore"
          />
          <CustomButton
            v-else
            type="primary"
            class="flex-1"
            :icon="Play"
            :text="t('dashboard.quickActions.startOpenListCore')"
            @click="toggleCore"
          />

          <CustomButton
            type="secondary"
            :disabled="!isCoreRunning || isCoreLoading"
            :icon="ExternalLink"
            class="flex-1"
            :text="t('dashboard.quickActions.openWeb')"
            @click="openWebUI"
          />

          <CustomButton
            type="secondary"
            :icon="Key"
            text=""
            :title="t('dashboard.quickActions.copyAdminPassword')"
            @click="copyAdminPassword"
          />
          <CustomButton
            type="secondary"
            :icon="RotateCcw"
            text=""
            :title="t('dashboard.quickActions.resetAdminPassword')"
            @click="resetAdminPassword"
          />
          <CustomButton
            v-if="isWindows"
            type="custom"
            :class="{
              'bg-success/80 hover:bg-success! text-white': !firewallEnabled,
              'bg-danger/80 hover:bg-danger! text-white': firewallEnabled,
            }"
            text-class="text-white"
            :disabled="firewallLoading"
            :icon="firewallLoading ? Loader : Shield"
            :text="
              firewallEnabled
                ? t('dashboard.quickActions.firewall.disable')
                : t('dashboard.quickActions.firewall.enable')
            "
            @click="toggleFirewallRule"
          />
        </div>
      </div>
      <div
        class="flex flex-wrap flex-col flex-1 gap-2 border p-2 rounded-md border-border-secondary bg-surface shadow-sm"
      >
        <div class="flex flex-wrap gap-2 items-center">
          <h4 class="text-main font-semibold text-sm">{{ t('dashboard.quickActions.rclone') }}</h4>
        </div>
        <div class="flex flex-wrap gap-2 items-center w-full">
          <CustomButton
            type="secondary"
            :icon="Settings"
            class="flex-1!"
            :text="t('dashboard.quickActions.configRclone')"
            @click="RouteToRcConfig"
          />
          <CustomButton
            type="secondary"
            :icon="HardDrive"
            class="flex-1!"
            :text="t('dashboard.quickActions.manageMounts')"
            @click="RouteToMounts"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ExternalLink, HardDrive, Key, Loader, Play, RotateCcw, Settings, Shield, Square } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { TauriAPI } from '@/api/tauri'
import useMessage from '@/hooks/useMessage'
import { createNewWindow, getAdminPassword } from '@/utils/common'
import { isMacOs, isWindows } from '@/utils/constant'

import { useTranslation } from '../../composables/useI18n'
import { useAppStore } from '../../stores/app'
import CustomButton from '../common/CustomButton.vue'

const { t } = useTranslation()
const router = useRouter()
const message = useMessage()
const appStore = useAppStore()
const firewallEnabled = ref(false)
const firewallLoading = ref(isWindows)
const { isCoreRunning, isCoreLoading } = storeToRefs(appStore)

const toggleCore = async () => {
  try {
    isCoreRunning.value ? await appStore.stopOpenListCore() : await appStore.startOpenListCore()
  } catch (error) {
    console.error('Failed to toggle OpenList Core:', error)
  }
}

const openWebUI = () => {
  if (!appStore.openListCoreUrl) return
  openLink(appStore.openListCoreUrl)
}

const RouteToRcConfig = () => {
  router.push({ name: 'Settings', query: { tab: 'rclone' } })
}

const RouteToMounts = () => {
  router.push({ name: 'Mount' })
}

const copyAdminPassword = async () => {
  const password = await getAdminPassword()
  if (password) {
    await navigator.clipboard.writeText(password)
    message.success(t('dashboard.quickActions.copyAdminPasswordSuccess'))
  } else {
    message.error(t('dashboard.quickActions.copyAdminPasswordFailed'))
  }
}

const resetAdminPassword = async () => {
  const newPassword = await appStore.resetAdminPassword()
  if (newPassword) {
    await navigator.clipboard.writeText(newPassword)
    message.success(t('dashboard.quickActions.resetAdminPasswordSuccess'))
  } else {
    message.error(t('dashboard.quickActions.resetAdminPasswordFailed'))
  }
}

const checkFirewallStatus = async () => {
  if (!isWindows) return
  try {
    firewallEnabled.value = await TauriAPI.firewall.check()
  } catch (error) {
    console.error('Failed to check firewall status:', error)
  } finally {
    firewallLoading.value = false
  }
}

const toggleFirewallRule = async () => {
  if (!isWindows) return
  try {
    firewallLoading.value = true
    if (firewallEnabled.value) {
      if (!(await TauriAPI.firewall.remove())) throw new Error('netsh command failed')
      firewallEnabled.value = false
      message.success(t('dashboard.quickActions.firewall.removed'))
    } else {
      if (!(await TauriAPI.firewall.add())) throw new Error('netsh command failed')
      firewallEnabled.value = true
      message.success(t('dashboard.quickActions.firewall.added'))
    }
  } catch (error: any) {
    console.error('Failed to toggle firewall rule:', error)
    const msg = firewallEnabled.value
      ? t('dashboard.quickActions.firewall.failedToRemove')
      : t('dashboard.quickActions.firewall.failedToAdd')
    message.error(msg + ': ' + (error.message || error))
  } finally {
    firewallLoading.value = false
  }
}

const openLink = async (url: string) => {
  if (appStore.settings.app.open_links_in_browser || isMacOs) {
    try {
      await TauriAPI.files.urlInBrowser(url)
    } catch (error) {
      console.error('Failed to open link:', error)
    }
    return
  }
  createNewWindow(url, `webview-${Date.now()}`, 'External Link')
}

onMounted(async () => {
  checkFirewallStatus()
})
</script>
