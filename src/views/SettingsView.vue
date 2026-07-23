<template>
  <div class="relative flex h-full w-full items-center justify-center">
    <div class="relative z-1 flex h-full w-full flex-col items-center justify-start gap-4 rounded-xl border-none p-4">
      <div
        class="flex w-full items-center justify-between gap-4 rounded-2xl border border-border-secondary px-2 py-1 shadow-md max-md:items-stretch max-md:p-5"
      >
        <div class="flex flex-1 flex-wrap items-center gap-4 p-2">
          <Settings :size="24" class="text-accent" />
          <div>
            <h1 class="m-0 text-xl font-semibold tracking-tight text-main">{{ t('settings.title') }}</h1>
            <p class="m-0 text-xs text-secondary">{{ t('settings.subtitle') }}</p>
          </div>
        </div>
        <div class="flex gap-3">
          <CustomButton type="secondary" :icon="RotateCcw" :text="t('common.reset')" @click="handleReset" />
          <CustomButton
            :disabled="isSaving"
            type="primary"
            :icon="Save"
            :text="isSaving ? t('common.saving') : t('settings.saveChanges')"
            @click="handleSave"
          />
        </div>
      </div>

      <div
        class="flex w-full items-center justify-between gap-4 rounded-2xl border border-border-secondary px-6 py-2 shadow-md max-md:items-stretch max-md:p-5"
      >
        <CustomButton
          v-for="tab in tabs"
          :key="tab.id"
          :text="tab.label"
          :icon="tab.icon"
          :icon-size="18"
          type="tab"
          :active="activeTab === tab.id"
          @click="activeTab = tab.id"
        />
      </div>

      <div
        class="relative flex h-full w-full flex-1 items-center justify-center overflow-hidden rounded-2xl border border-border-secondary p-1 shadow-md"
      >
        <div
          v-if="activeTab === 'openlist'"
          class="no-scrollbar flex h-full w-full flex-1 flex-col gap-6 overflow-auto p-4"
        >
          <SettingSection :icon="Server" :title="t('settings.network.title')">
            <SettingCard>
              <CustomInput
                v-model.number="openlistCoreSettings.port"
                type="number"
                :placeholder="t('settings.service.network.port.placeholder')"
                :tips="t('settings.service.network.port.help')"
                :title="t('settings.service.network.port.label')"
                :min="1"
                :max="65535"
              />
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="openlistCoreSettings.data_dir"
                type="text"
                :placeholder="t('settings.service.network.dataDir.placeholder')"
                :tips="t('settings.service.network.dataDir.help')"
                :title="t('settings.service.network.dataDir.label')"
              >
                <template #input-extra>
                  <div class="flex gap-2 mt-1">
                    <CustomButton
                      type="secondary"
                      text=""
                      :icon="FolderOpen"
                      :title="t('settings.service.network.dataDir.selectTitle')"
                      @click="handleSelectDataDir"
                    />
                    <CustomButton
                      type="secondary"
                      text=""
                      :icon="ExternalLink"
                      :title="t('settings.service.network.dataDir.openTitle')"
                      @click="handleOpenDataDir"
                    />
                  </div>
                </template>
              </CustomInput>
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="openlistCoreSettings.binary_path"
                type="text"
                :placeholder="t('settings.service.customPaths.openlistBinary.placeholder')"
                :tips="t('settings.service.customPaths.openlistBinary.help')"
                :title="t('settings.service.customPaths.openlistBinary.label')"
              >
                <template #input-extra>
                  <div class="flex gap-2 mt-1">
                    <CustomButton
                      type="secondary"
                      text=""
                      :icon="FolderOpen"
                      :title="t('settings.service.customPaths.openlistBinary.selectTitle')"
                      @click="handleSelectOpenlistBinary"
                    />
                  </div>
                </template>
              </CustomInput>
            </SettingCard>
            <SettingCard p1 class="flex items-center">
              <CustomSwitch
                v-model="openlistCoreSettings.ssl_enabled"
                :title="t('settings.service.network.ssl.title')"
                no-border
                small
                class="w-full"
                :tips="t('settings.service.network.ssl.description')"
              />
            </SettingCard>
          </SettingSection>

          <SettingSection :icon="Settings2Icon" :title="t('settings.common')" only-one-row>
            <SettingCard p1>
              <CustomSwitch
                v-model="openlistCoreSettings.auto_launch"
                :title="t('settings.service.startup.autoLaunch.title')"
                no-border
                small
                :tips="t('settings.service.startup.autoLaunch.description')"
              />
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="appSettings.admin_password"
                type="text"
                :placeholder="t('settings.service.admin.passwordPlaceholder')"
                :tips="t('settings.service.admin.help')"
                :title="t('settings.service.admin.title')"
              >
                <template #input-extra>
                  <div class="flex gap-2 mt-1">
                    <CustomButton
                      type="secondary"
                      text=""
                      :disabled="isResettingPassword"
                      :icon="RotateCcw"
                      :title="t('settings.service.admin.resetTitle')"
                      @click="handleResetAdminPassword"
                    />
                  </div>
                </template>
              </CustomInput>
            </SettingCard>
          </SettingSection>
        </div>

        <div
          v-if="activeTab === 'rclone'"
          class="no-scrollbar flex h-full w-full flex-1 flex-col gap-6 overflow-auto p-4"
        >
          <SettingSection :icon="Package" :title="t('settings.service.customPaths.title')">
            <SettingCard>
              <CustomInput
                v-model="rcloneSettings.binary_path"
                type="text"
                :placeholder="t('settings.service.customPaths.rcloneBinary.placeholder')"
                :tips="t('settings.service.customPaths.rcloneBinary.help')"
                :title="t('settings.service.customPaths.rcloneBinary.label')"
              >
                <template #input-extra>
                  <div class="flex gap-2 mt-1">
                    <CustomButton
                      type="secondary"
                      text=""
                      :icon="FolderOpen"
                      :title="t('settings.service.customPaths.rcloneBinary.selectTitle')"
                      @click="handleSelectRcloneBinary"
                    />
                  </div>
                </template>
              </CustomInput>
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="rcloneSettings.rclone_conf_path"
                type="text"
                :placeholder="t('settings.service.customPaths.rcloneConfig.placeholder')"
                :tips="t('settings.service.customPaths.rcloneConfig.help')"
                :title="t('settings.service.customPaths.rcloneConfig.label')"
              >
                <template #input-extra>
                  <div class="flex gap-2 mt-1">
                    <CustomButton
                      type="secondary"
                      text=""
                      :icon="FolderOpen"
                      :title="t('settings.service.customPaths.rcloneConfig.selectTitle')"
                      @click="handleSelectRcloneConfig"
                    />
                  </div>
                </template>
              </CustomInput>
            </SettingCard>
          </SettingSection>
          <SettingSection :icon="SaveIcon" :title="t('settings.rclone.config.subtitle')" only-one-row>
            <div class="flex flex-col gap-4">
              <CustomButton
                type="secondary"
                :icon="FileIcon"
                :text="t('settings.rclone.config.openFile')"
                @click="handleOpenRcloneConfig"
              />
            </div>
          </SettingSection>
        </div>

        <div v-if="activeTab === 'app'" class="no-scrollbar flex h-full w-full flex-1 flex-col gap-6 overflow-auto p-4">
          <SettingSection :icon="ExternalLink" :title="t('settings.common')">
            <SettingCard p1>
              <CustomSwitch
                v-model="appSettings.open_links_in_browser"
                :title="t('settings.app.links.openInBrowser.title')"
                no-border
                small
                :tips="t('settings.app.links.openInBrowser.description')"
              />
            </SettingCard>
            <CustomNavCard
              type="secondary"
              :icon="ExternalLink"
              :title="t('settings.app.config.subtitle')"
              @click="handleOpenSettingsFile"
            />
            <SettingCard p1>
              <CustomSwitch
                v-model="autoStartApp"
                :title="t('settings.app.autoStartApp.title')"
                no-border
                small
                class="w-full"
                :tips="t('settings.app.autoStartApp.description')"
              />
            </SettingCard>
            <SettingCard p1>
              <CustomSwitch
                v-model="appSettings.show_window_on_startup"
                :title="t('settings.app.showWindowOnStartup.title')"
                no-border
                small
                class="w-full"
                :tips="t('settings.app.showWindowOnStartup.description')"
              />
            </SettingCard>
            <SettingCard v-if="isMacOs" p1>
              <CustomSwitch
                v-model="appSettings.hide_dock_icon"
                :title="t('settings.app.hideDockIcon.title')"
                no-border
                small
                class="w-full"
                :tips="t('settings.app.hideDockIcon.description')"
              />
            </SettingCard>
            <SettingCard>
              <SingleSelect
                v-model="currentLanguage"
                :key-list="languageOptions.map(item => item.value)"
                :title="t('settings.app.language.title')"
                :fronticon="false"
                :tight="false"
                :placeholder="languageOptions.find(item => item.value === currentLanguage)?.label || currentLanguage"
              >
                <template #item="{ item }">
                  {{ languageOptions.find(opt => opt.value === item)?.label || item }}
                </template>
              </SingleSelect>
            </SettingCard>
          </SettingSection>

          <SettingSection :icon="Github" :title="t('settings.app.ghProxy.title')">
            <SettingCard>
              <CustomInput
                v-model="appSettings.gh_proxy"
                type="text"
                :placeholder="t('settings.app.ghProxy.placeholder')"
                :tips="t('settings.app.ghProxy.help')"
                :title="t('settings.app.ghProxy.label')"
              />
            </SettingCard>
            <SettingCard p1 class="flex items-center">
              <CustomSwitch
                v-model="appSettings.gh_proxy_api"
                :title="t('settings.app.ghProxy.api.title')"
                no-border
                small
                class="w-full"
                :tips="t('settings.app.ghProxy.api.description')"
              />
            </SettingCard>
          </SettingSection>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { open } from '@tauri-apps/plugin-dialog'
import {
  ExternalLink,
  FileIcon,
  FolderOpen,
  Github,
  HardDrive,
  Package,
  RotateCcw,
  Save,
  SaveIcon,
  Server,
  Settings,
  Settings2Icon,
} from 'lucide-vue-next'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import CustomInput from '@/components/common/CustomInput.vue'
import CustomNavCard from '@/components/common/CustomNavCard.vue'
import CustomSwitch from '@/components/common/CustomSwitch.vue'
import SettingCard from '@/components/common/SettingCard.vue'
import SettingSection from '@/components/common/SettingSection.vue'
import SingleSelect from '@/components/common/SingleSelect.vue'
import useConfirm from '@/hooks/useConfirm'
import useMessage from '@/hooks/useMessage'
import { getAdminPassword } from '@/utils/common'
import { isMacOs } from '@/utils/constant'
import { DEFAULT_CONFIG } from '@/utils/constant'

import CustomButton from '../components/common/CustomButton.vue'
import { useTranslation } from '../composables/useI18n'
import { useAppStore } from '../stores/app'

const appStore = useAppStore()
const route = useRoute()
const { t, currentLocale, switchLanguage } = useTranslation()
const isSaving = ref(false)
const message = useMessage()
const confirm = useConfirm()
const activeTab = ref('openlist')
const autoStartApp = ref(false)
const isResettingPassword = ref(false)

const languageOptions = [
  { label: '中文', value: 'zh' },
  { label: 'English', value: 'en' },
]

const currentLanguage = ref(currentLocale.value)

watch(currentLanguage, newLang => {
  switchLanguage(newLang)
})

const openlistCoreSettings = reactive({ ...appStore.settings.openlist })
const rcloneSettings = reactive({ ...appStore.settings.rclone })
const appSettings = reactive({ ...appStore.settings.app })
let originalOpenlistPort = openlistCoreSettings.port || 5244
let originalDataDir = openlistCoreSettings.data_dir
let originalOpenListBinaryPath = openlistCoreSettings.binary_path || ''
let originalAdminPassword = appStore.settings.app.admin_password || ''

const tabs = computed(() => [
  {
    id: 'openlist',
    label: t('settings.tabs.openlist'),
    icon: Server,
    description: t('settings.service.subtitle'),
  },
  {
    id: 'rclone',
    label: t('settings.tabs.rclone'),
    icon: HardDrive,
    description: t('settings.rclone.subtitle'),
  },
  {
    id: 'app',
    label: t('settings.tabs.app'),
    icon: Settings,
    description: t('settings.app.subtitle'),
  },
])

watch(autoStartApp, async newValue => {
  if (newValue) {
    await enable()
  } else {
    await disable()
  }
})

const handleSave = async () => {
  isSaving.value = true

  try {
    appStore.settings.openlist = { ...openlistCoreSettings }
    appStore.settings.rclone = { ...rcloneSettings }
    appStore.settings.app = { ...appSettings }

    const needsPasswordUpdate = originalAdminPassword !== appSettings.admin_password && appSettings.admin_password

    if (
      originalOpenlistPort !== openlistCoreSettings.port ||
      originalDataDir !== (openlistCoreSettings.data_dir || '') ||
      originalOpenListBinaryPath !== (openlistCoreSettings.binary_path || '')
    ) {
      await appStore.saveAndRestart()
    } else {
      await appStore.saveSettings()
    }

    if (needsPasswordUpdate && appSettings.admin_password) {
      const res = await appStore.setAdminPassword(appSettings.admin_password)
      if (res) {
        originalAdminPassword = appSettings.admin_password
        message.success(t('settings.service.admin.passwordUpdated'))
      } else {
        message.error(t('settings.service.admin.passwordUpdateFailed'))
      }
    } else {
      message.success(t('settings.saved'))
    }

    originalOpenlistPort = openlistCoreSettings.port || 5244
    originalDataDir = openlistCoreSettings.data_dir
    originalOpenListBinaryPath = openlistCoreSettings.binary_path || ''
  } catch (error) {
    message.error(t('settings.saveFailed'))
    console.error('Save settings error:', error)
  } finally {
    isSaving.value = false
  }
}

const handleReset = async () => {
  const result = await confirm.confirm({
    message: t('settings.confirmReset.message'),
    title: t('settings.confirmReset.title'),
    confirmButtonText: t('common.confirm'),
    cancelButtonText: t('common.cancel'),
    type: 'warning',
  })
  if (!result) {
    return
  }
  try {
    await appStore.resetSettings()
    Object.assign(openlistCoreSettings, appStore.settings.openlist)
    Object.assign(rcloneSettings, appStore.settings.rclone)
    Object.assign(appSettings, appStore.settings.app)
    message.info(t('settings.resetSuccess'))
  } catch (_error) {
    message.error(t('settings.resetFailed'))
  }
}

const handleSelectDataDir = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.service.network.dataDir.selectTitle'),
      defaultPath: openlistCoreSettings.data_dir || undefined,
    })

    if (selected && typeof selected === 'string') {
      openlistCoreSettings.data_dir = selected
    }
  } catch (error) {
    console.error('Failed to select directory:', error)

    message.error(t('settings.service.network.dataDir.selectError'))
  }
}

const handleOpenDataDir = async () => {
  try {
    await appStore.openOpenListDataDir()
    message.success(t('settings.service.network.dataDir.openSuccess'))
  } catch (error) {
    console.error('Failed to open data directory:', error)
    message.error(t('settings.service.network.dataDir.openError'))
  }
}

const handleResetAdminPassword = async () => {
  isResettingPassword.value = true
  const newPassword = await appStore.resetAdminPassword()
  if (newPassword) {
    appSettings.admin_password = newPassword
    message.success(t('settings.service.admin.resetSuccess'))
  } else {
    message.error(t('settings.service.admin.resetFailed'))
  }
  isResettingPassword.value = false
}

const handleOpenRcloneConfig = async () => {
  try {
    await appStore.openRcloneConfigFile()
    message.success(t('settings.rclone.config.openSuccess'))
  } catch (error) {
    console.error('Failed to open rclone config file:', error)
    message.error(t('settings.rclone.config.openError'))
  }
}

const handleOpenSettingsFile = async () => {
  try {
    await appStore.openSettingsFile()
    message.success(t('settings.app.config.openSuccess'))
  } catch (error) {
    console.error('Failed to open settings file:', error)
    message.error(t('settings.app.config.openError'))
  }
}

const handleSelectOpenlistBinary = async () => {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      title: t('settings.service.customPaths.openlistBinary.selectTitle'),
      defaultPath: openlistCoreSettings.binary_path || undefined,
      filters: [
        {
          name: 'Executable',
          extensions: OS_PLATFORM === 'win32' ? ['exe'] : ['*'],
        },
      ],
    })

    if (selected && typeof selected === 'string') {
      openlistCoreSettings.binary_path = selected
    }
  } catch (error) {
    console.error('Failed to select OpenList binary:', error)
    message.error(t('settings.service.customPaths.selectError'))
  }
}

const handleSelectRcloneBinary = async () => {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      title: t('settings.service.customPaths.rcloneBinary.selectTitle'),
      defaultPath: rcloneSettings.binary_path || undefined,
      filters: [
        {
          name: 'Executable',
          extensions: OS_PLATFORM === 'win32' ? ['exe'] : ['*'],
        },
      ],
    })

    if (selected && typeof selected === 'string') {
      rcloneSettings.binary_path = selected
    }
  } catch (error) {
    console.error('Failed to select Rclone binary:', error)
    message.error(t('settings.service.customPaths.selectError'))
  }
}

const handleSelectRcloneConfig = async () => {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      title: t('settings.service.customPaths.rcloneConfig.selectTitle'),
      defaultPath: rcloneSettings.rclone_conf_path || undefined,
      filters: [
        {
          name: 'Config',
          extensions: ['conf', '*'],
        },
      ],
    })

    if (selected && typeof selected === 'string') {
      rcloneSettings.rclone_conf_path = selected
    }
  } catch (error) {
    console.error('Failed to select Rclone config:', error)
    message.error(t('settings.service.customPaths.selectError'))
  }
}

const loadCurrentAdminPassword = async () => {
  try {
    const password = await getAdminPassword()
    if (password) {
      appSettings.admin_password = password
      originalAdminPassword = password
    }
  } catch (error) {
    console.error('Failed to load admin password:', error)
  }
}

async function init() {
  autoStartApp.value = await isEnabled()
  const tabParam = route.query.tab as string
  if (tabParam && ['openlist', 'rclone', 'app'].includes(tabParam)) {
    activeTab.value = tabParam
  }
  Object.assign(openlistCoreSettings, { ...DEFAULT_CONFIG.openlistCore, ...openlistCoreSettings })
  Object.assign(rcloneSettings, { ...DEFAULT_CONFIG.rclone, ...rcloneSettings })
  Object.assign(appSettings, { ...DEFAULT_CONFIG.app, ...appSettings })

  originalOpenlistPort = openlistCoreSettings.port || 5244
  originalDataDir = openlistCoreSettings.data_dir
  originalOpenListBinaryPath = openlistCoreSettings.binary_path || ''
  // Load current admin password
  await loadCurrentAdminPassword()
}

onMounted(async () => {
  await init()
})
</script>
