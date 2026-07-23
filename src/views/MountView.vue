<template>
  <div class="relative flex h-full w-full items-center justify-center">
    <!-- Header Section -->
    <div class="relative z-1 flex h-full w-full flex-col items-center justify-start gap-4 rounded-xl border-none p-4">
      <div
        class="flex w-full items-center justify-between gap-4 rounded-2xl border border-border-secondary px-2 py-1 shadow-sm"
      >
        <div class="flex flex-1 flex-wrap items-center gap-4 p-2">
          <HardDrive class="text-accent" />
          <div class="flex flex-col gap-0.5">
            <h1 class="m-0 text-xl font-semibold tracking-tight text-main">{{ t('mount.title') }}</h1>
            <span class="text-sm text-secondary">{{
              `${configCounts.mounted} / ${configCounts.total} ${t('mount.stats.mounted')}`
            }}</span>
          </div>
        </div>
        <div class="flex gap-3">
          <CustomButton type="secondary" :icon="RefreshCw" :text="t('mount.actions.refresh')" @click="refreshData" />
          <CustomButton type="primary" :icon="Plus" :text="t('mount.actions.addRemote')" @click="addNewConfig" />
        </div>
      </div>

      <div
        v-if="shouldShowWebdavTip"
        class="flex w-full items-center border border-border-secondary bg-warning/10 p-3 rounded-md gap-3"
      >
        <div>
          <Settings class="text-warning" />
        </div>
        <div class="flex flex-1 flex-col gap-0.5">
          <h4 class="text-main text-sm font-semibold">{{ t('mount.tip.webdavTitle') }}</h4>
          <p class="text-xs font-medium text-secondary select-text">{{ t('mount.tip.webdavMessage') }}</p>
        </div>
        <button
          class="flex bg-danger/50 rounded-full p-1 hover:bg-danger transition-colors duration-200 ease-apple"
          :title="t('mount.tip.dismissForever')"
          @click="dismissWebdavTip"
        >
          <X class="text-white" :size="14" />
        </button>
      </div>

      <div
        v-if="showWinfspTip"
        class="flex w-full items-center border border-border-secondary bg-warning/10 p-3 rounded-md gap-3"
      >
        <div>
          <HardDrive class="text-warning" />
        </div>
        <div class="flex flex-1 flex-col gap-0.5">
          <h4 class="text-main text-sm font-semibold">{{ t('mount.tip.winfspTitle') }}</h4>
          <p class="text-xs font-medium text-secondary select-text">{{ t('mount.tip.winfspMessage') }}</p>
        </div>
        <button
          class="flex bg-danger/50 rounded-full p-1 hover:bg-danger transition-colors duration-200 ease-apple"
          :title="t('mount.tip.dismissForever')"
          @click="dismissWinfspTip"
        >
          <X class="text-white" :size="14" />
        </button>
      </div>

      <div
        v-if="showRcloneTip"
        class="flex w-full items-center border border-border-secondary bg-warning/10 p-3 rounded-md gap-3"
      >
        <div>
          <HardDrive class="text-warning" />
        </div>
        <div class="flex flex-1 flex-col gap-0.5">
          <h4 class="text-main text-sm font-semibold">{{ t('mount.tip.rcloneTitle') }}</h4>
          <p class="text-xs font-medium text-secondary select-text">{{ t('mount.tip.rcloneMessage') }}</p>
        </div>
        <button
          class="flex bg-danger/50 rounded-full p-1 hover:bg-danger transition-colors duration-200 ease-apple"
          :title="t('mount.tip.dismissForever')"
          @click="dismissRcloneTip"
        >
          <X class="text-white" :size="14" />
        </button>
      </div>

      <!-- Controls Section -->
      <div class="flex w-full border border-border-secondary shadow-sm p-2 rounded-xl justify-between gap-3">
        <div class="flex-1 flex items-center">
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t('mount.filters.searchPlaceholder')"
            class="border border-border rounded-md px-2 py-1 w-full text-sm text-main bg-surface focus:outline-none focus:border-accent"
          />
        </div>
        <div class="flex items-center gap-2">
          <SingleSelect
            v-model="statusFilter"
            :key-list="filterList.map(item => item.value)"
            title=""
            :fronticon="false"
            :placeholder="filterList.find(item => item.value === statusFilter)?.label || t('mount.filters.allStatus')"
          >
            <template #item="{ item }">
              {{ filterList.find(filterItem => filterItem.value === item)?.label || item }}
            </template>
          </SingleSelect>
        </div>
      </div>
      <!-- Error Display -->
      <div
        v-if="rcloneStore.error"
        class="flex w-full items-center gap-2 p-3 rounded-md bg-danger/10 border border-danger"
      >
        <XCircle class="text-danger" />
        <div class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
          <span class="text-secondary text-xs">{{ rcloneStore.error || '' }}</span>
        </div>
        <button
          class="rounded-full bg-danger/50 p-1 hover:bg-danger transition-colors duration-200 ease-apple"
          @click="rcloneStore.clearError"
        >
          <X class="text-white" :size="14" />
        </button>
      </div>

      <!-- Remote Configurations -->
      <div class="flex h-full w-full flex-1 flex-col gap-4 overflow-hidden rounded-md shadow-md">
        <div v-if="initLoading" class="flex w-full h-full overflow-auto no-scrollbar items-center justify-center">
          <div class="flex flex-col gap-4 w-full h-full items-center justify-center">
            <div class="border-3 border-border w-20 h-20 rounded-full border-t-accent border-t-3 animate-spin"></div>
            <div class="text-main font-semibold">{{ t('mount.loading') }}</div>
          </div>
        </div>
        <div
          v-else-if="filteredConfigs.length === 0 && !initLoading"
          class="flex w-full h-full overflow-auto items-center justify-center p-2"
        >
          <div class="max-w-80 flex flex-col items-center justify-center gap-2">
            <Cloud class="text-secondary w-12 h-12" />
            <h3 class="text-main font-semibold text-xl">{{ t('mount.empty.title') }}</h3>
            <p class="text-secondary text-sm">{{ t('mount.empty.description') }}</p>
          </div>
        </div>

        <div v-else class="w-full h-full overflow-auto no-scrollbar items-center justify-center">
          <div class="w-full h-auto grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4 p-4">
            <div
              v-for="config in filteredConfigs"
              :key="config.name"
              class="bg-surface rounded-xl border border-border-secondary p-4 shadow-sm flex flex-col justify-between h-full hover:border-2 hover:border-accent transition-all duration-200 ease-apple gap-2"
              :class="{
                'border-success/50! border-2': isConfigMounted(config.name) && !loadingList.includes(config.name),
                'border-danger/50! border-2':
                  getConfigStatus(config.name) === 'error' && !loadingList.includes(config.name),
                'border-warning/50! border-2': loadingList.includes(config.name),
              }"
            >
              <div class="flex items-start justify-between gap-1">
                <div class="flex items-start gap-3 flex-1">
                  <div class="flex items-center justify-center w-8 h-8 rounded-md bg-accent/10 text-accent shrink-0">
                    <Cloud
                      :class="{
                        'text-success': isConfigMounted(config.name) && !loadingList.includes(config.name),
                        'text-danger': getConfigStatus(config.name) === 'error' && !loadingList.includes(config.name),
                        'text-warning': loadingList.includes(config.name),
                      }"
                    />
                  </div>
                  <div class="flex-1 min-w-0">
                    <h3 class="text-sm font-semibold text-main/80">{{ config.name }}</h3>
                    <p
                      class="text-xs text-secondary whitespace-nowrap overflow-hidden text-ellipsis font-['SF_Mono',monospace] tracking-tighter"
                    >
                      {{ config.url }}
                    </p>
                  </div>
                </div>

                <div class="flex items-center justify-center w-6 h-6 shrink-0 bg-bg-secondary rounded-md">
                  <component
                    :is="loadingList.includes(config.name) ? Loader : getStatusIcon(getConfigStatus(config.name))"
                    class="w-4 h-4 text-secondary"
                    :class="{
                      'text-warning animate-spin': loadingList.includes(config.name),
                      'text-success': getConfigStatus(config.name) === 'mounted' && !loadingList.includes(config.name),
                      'text-error!': getConfigStatus(config.name) === 'error' && !loadingList.includes(config.name),
                    }"
                  />
                </div>
              </div>

              <div class="flex w-full flex-col gap-3 flex-1 justify-between">
                <div class="flex-1 flex items-center justify-start gap-2 flex-wrap">
                  <span
                    class="inline-flex items-center py-1 px-2 bg-bg-secondary rounded-sm text-[0.6rem] font-semibold text-secondary uppercase"
                    >{{ config.type }}</span
                  >

                  <span
                    v-if="config.volumeName"
                    class="inline-flex items-center py-1 px-2 bg-bg-secondary rounded-sm text-[0.6rem] font-semibold text-secondary uppercase"
                    >{{ config.volumeName }}</span
                  >
                  <span
                    v-if="config.autoMount"
                    class="inline-flex items-center py-1 px-2 bg-success/30 rounded-sm text-[0.6rem] font-semibold text-secondary uppercase"
                    >{{ t('mount.meta.autoMount') }}</span
                  >
                </div>
                <span
                  v-if="getConfigStatus(config.name) === 'error' && !loadingList.includes(config.name)"
                  class="group/badge overflow-hidden text-sm font-medium text-ellipsis whitespace-nowrap text-secondary bg-error/10 rounded-md py-0.5 px-1"
                >
                  <div class="min-w-0 flex-1 overflow-hidden">
                    <div
                      class="flex overflow-hidden text-ellipsis whitespace-nowrap group-hover/badge:w-fit group-hover/badge:animate-[badge-scroll_5s_linear_infinite] group-hover/badge:text-clip"
                    >
                      <span class="text-xs py-0.5 leading-none whitespace-nowrap group-hover/badge:pr-5">{{
                        getErrorMsg(config.name)
                      }}</span>
                      <span class="text-xs py-0.5 hidden leading-none whitespace-nowrap group-hover/badge:block">{{
                        getErrorMsg(config.name)
                      }}</span>
                    </div>
                  </div>
                </span>
              </div>

              <div class="flex items-center justify-between gap-3">
                <div class="flex-1">
                  <CustomButton
                    v-if="!isConfigMounted(config.name)"
                    :disabled="loadingList.includes(config.name) || !config.mountPoint"
                    :title="!config.mountPoint ? t('mount.messages.mountPointRequired') : ''"
                    type="primary"
                    class="flex-1 w-full"
                    :icon="Play"
                    :text="t('mount.actions.mount')"
                    :icon-size="14"
                    @click="mountConfig(config)"
                  />
                  <CustomButton
                    v-else
                    type="danger"
                    class="flex-1 w-full bg-warning/70 hover:bg-warning!"
                    icon-class="text-white"
                    text-class="text-white"
                    :icon="Square"
                    :disabled="loadingList.includes(config.name)"
                    :text="t('mount.actions.unmount')"
                    :icon-size="14"
                    @click="unmountConfig(config)"
                  />
                </div>

                <div class="flex gap-2">
                  <CustomButton
                    :icon="Edit"
                    :title="t('mount.actions.edit')"
                    text=""
                    :icon-size="14"
                    type="secondary"
                    @click="editConfig(config)"
                  />
                  <CustomButton
                    :icon="Trash2"
                    :title="t('mount.actions.delete')"
                    text=""
                    :disabled="isConfigMounted(config.name) || loadingList.includes(config.name)"
                    :icon-size="14"
                    icon-class="text-white"
                    class="bg-danger/50 hover:bg-danger!"
                    type="custom"
                    @click="deleteConfig(config)"
                  />
                  <CustomButton
                    v-if="isConfigMounted(config.name)"
                    :icon="FolderOpen"
                    :title="t('mount.actions.openInExplorer')"
                    text=""
                    :icon-size="14"
                    type="secondary"
                    class="border-none!"
                    @click="openInFileExplorer(config.mountPoint)"
                  />
                  <CustomButton
                    v-if="getConfigStatus(config.name) === 'error' || loadingList.includes(config.name)"
                    :icon="X"
                    :title="t('mount.actions.stopProcess')"
                    text=""
                    :icon-size="14"
                    type="secondary"
                    class="border-none!"
                    @click="stopProcess(config.name)"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!-- Configuration Modal -->
    <CustomModal
      v-show="showAddForm"
      v-model:visible="showAddForm"
      :title="editingConfig ? t('mount.config.editTitle') : t('mount.config.addTitle')"
      @close="resetForm"
    >
      <div class="flex-1 w-full overflow-auto no-scrollbar p-4">
        <div class="flex flex-col gap-6">
          <SettingSection :icon="Globe2Icon" :title="t('mount.config.basicInfo')">
            <SettingCard>
              <CustomInput
                v-model="configForm.name"
                :title="t('mount.config.name')"
                :required="true"
                :disabled="!isAddingNew"
                :placeholder="t('mount.config.namePlaceholder')"
              />
            </SettingCard>
            <SettingCard>
              <SingleSelect
                v-model="configForm.type"
                :key-list="['webdav']"
                :title="t('mount.config.type')"
                :required="true"
                disabled
                :tight="false"
                :fronticon="false"
                placeholder="WebDAV"
              >
                <template #item="{ item }">
                  <span>
                    {{ item === 'webdav' ? t('mount.config.types.webdav') : item }}
                  </span>
                </template>
              </SingleSelect>
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="configForm.url"
                :title="t('mount.config.url')"
                :required="true"
                :placeholder="t('mount.config.urlPlaceholder')"
              />
            </SettingCard>

            <SettingCard>
              <CustomInput
                v-model="configForm.vendor"
                type="text"
                :title="t('mount.config.vendor')"
                :placeholder="t('mount.config.vendorPlaceholder')"
              />
            </SettingCard>
          </SettingSection>

          <SettingSection :icon="ShieldUser" :title="t('mount.config.authentication')">
            <SettingCard>
              <CustomInput
                v-model="configForm.user"
                :title="t('mount.config.username')"
                :required="true"
                :placeholder="t('mount.config.usernamePlaceholder')"
              />
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="configForm.pass"
                type="text"
                :title="t('mount.config.password')"
                :placeholder="t('mount.config.passwordPlaceholder')"
              />
            </SettingCard>
          </SettingSection>

          <SettingSection :icon="Database" :title="t('mount.config.mountSettings')">
            <SettingCard>
              <CustomInput
                v-model="configForm.mountPoint"
                type="text"
                :title="t('mount.config.mountPoint')"
                :placeholder="t('mount.config.mountPointPlaceholder')"
              />
            </SettingCard>
            <SettingCard>
              <CustomInput
                v-model="configForm.volumeName"
                type="text"
                :title="t('mount.config.volumeName')"
                :placeholder="t('mount.config.volumeNamePlaceholder')"
              />
            </SettingCard>
            <SettingCard>
              <CustomSwitch
                v-model="configForm.autoMount"
                :title="t('mount.config.autoMount')"
                class="w-full"
                no-border
                small
              />
            </SettingCard>
          </SettingSection>

          <SettingSection :icon="Settings" :title="t('mount.config.extraFlags')" only-one-row>
            <div class="flex flex-col items-center justify-center w-full gap-4">
              <div class="flex items-center justify-between w-full gap-3">
                <CustomButton
                  type="secondary"
                  :text="t('mount.config.quickFlags')"
                  :icon="Settings"
                  class="flex-1 bg-accent/10! hover:bg-accent/20!"
                  :title="t('mount.config.quickFlagsTooltip')"
                  @click="showFlagSelector = !showFlagSelector"
                />
                <CustomButton
                  type="primary"
                  :text="t('mount.config.addFlag')"
                  :icon="Plus"
                  class="flex-1"
                  @click="addFlag"
                />
              </div>
              <!-- Manual Flags Input -->
              <div class="flex flex-col w-full gap-2">
                <div
                  v-for="(_, index) in configForm.extraFlags || []"
                  :key="index"
                  class="flex items-center gap-2 w-full"
                >
                  <input
                    v-model="configForm.extraFlags![index]"
                    type="text"
                    class="flex-1 py-2 px-3 border border-border-secondary rounded-md bg-surface text-sm text-main focus:outline-none focus:border-accent"
                    :placeholder="t('mount.config.flagPlaceholder')"
                  />
                  <CustomButton
                    type="secondary"
                    :icon="X"
                    text=""
                    :title="t('mount.config.removeFlag')"
                    @click="removeFlag(index)"
                  />
                </div>
              </div>
              <div v-if="showFlagSelector" class="flex w-full">
                <div class="flex-1 p-2">
                  <div class="bg-accent/10 border border-t-2 border-accent border-t-accent rounded-md p-2 text-center">
                    <p class="text-sm font-semibold text-secondary">{{ t('mount.config.clickToToggleFlags') }}</p>
                  </div>

                  <div class="grid grid-cols-[repeat(auto-fit,minmax(250px,1fr))] gap-4 mt-4">
                    <div
                      v-for="category in commonFlags"
                      :key="category.category"
                      class="border border-border-secondary rounded-md p-3 flex-col flex gap-3 shadow-md bg-surface"
                    >
                      <div
                        class="flex items-center justify-center border-b-2 border-b-border bg-accent/10 rounded-sm p-1"
                      >
                        <h5 class="text-main text-sm font-semibold">
                          {{ t(`mount.config.flagCategories.${category.category}`) }}
                        </h5>
                      </div>

                      <div
                        v-for="flag in category.flags"
                        :key="`${flag.flag}-${flag.value}`"
                        class="flex items-center rounded-md gap-2 py-2 px-2.5 border-none bg-bg text-main text-left cursor-pointer border-b border-b-border relative last:border-b-0 hover:bg-accent/10"
                        :class="{
                          'bg-accent/20!': isFlagInConfig(flag),
                        }"
                        :title="getFlagDescription(flag)"
                        @click="toggleFlag(flag)"
                      >
                        <div class="flex items-center shrink-0">
                          <div
                            class="w-5.5 h-5.5 border-2 border-border rounded-md bg-bg flex items-center justify-center cursor-pointer relative hover:border-accent"
                            :class="{ 'bg-success/10! border-success/10!': isFlagInConfig(flag) }"
                          >
                            <CheckCircle v-if="isFlagInConfig(flag)" class="w-3.5 h-3.5 text-success stroke-[3px]" />
                          </div>
                        </div>
                        <div class="flex flex-col gap-2 flex-1 min-w-0">
                          <code
                            class="font-['SF_Mono',Consolas,Monaco,'Courier_New',monospace] text-xs font-semibold py-1.5 px-2.5 bg-bg-tertiary border border-border rounded-md text-accent inline-block max-w-fit leading-[1.2] tracking-wider"
                            >{{ flag.flag }}{{ flag.value ? `=${flag.value}` : '' }}</code
                          >
                          <span class="text-xs font-medium text-secondary leading-normal m-0">{{
                            getFlagDescription(flag)
                          }}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </SettingSection>
        </div>
      </div>

      <template #footer>
        <CustomButton :icon="X" :text="t('common.cancel')" type="secondary" @click="cancelForm" />
        <CustomButton :icon="Save" :text="editingConfig ? t('common.save') : t('common.add')" @click="saveConfig" />
      </template>
    </CustomModal>
  </div>
</template>

<script setup lang="ts">
import {
  CheckCircle,
  Cloud,
  Database,
  Edit,
  FolderOpen,
  Globe2Icon,
  HardDrive,
  Loader,
  Play,
  Plus,
  RefreshCw,
  Save,
  Settings,
  ShieldUser,
  Square,
  Trash2,
  X,
  XCircle,
} from 'lucide-vue-next'
import { computed, ComputedRef, onMounted, onUnmounted, Ref, ref } from 'vue'

import CustomButton from '@/components/common/CustomButton.vue'
import CustomInput from '@/components/common/CustomInput.vue'
import CustomModal from '@/components/common/CustomModal.vue'
import CustomSwitch from '@/components/common/CustomSwitch.vue'
import SettingCard from '@/components/common/SettingCard.vue'
import SettingSection from '@/components/common/SettingSection.vue'
import SingleSelect from '@/components/common/SingleSelect.vue'
import useConfirm from '@/hooks/useConfirm'
import useMessage from '@/hooks/useMessage'
import { useAppStore } from '@/stores/app'
import { isLinux, isWindows } from '@/utils/constant'

import { useTranslation } from '../composables/useI18n'
import { useRcloneStore } from '../stores/rclone'

const { t } = useTranslation()
const rcloneStore = useRcloneStore()
const appStore = useAppStore()
const confirm = useConfirm()
const message = useMessage()

const showAddForm = ref(false)
const editingConfig = ref<RcloneFormConfig | null>(null)
const searchQuery = ref('')
const statusFilter = ref<'all' | 'mounted' | 'unmounted' | 'error'>('all')
const showFlagSelector = ref(false)
const isAddingNew = ref(false)
const initLoading = ref(true)
const loadingList = ref<string[]>([])
let mountRefreshInterval: NodeJS.Timeout | null = null

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
    'vfs-cache-mode': 'full',
  },
}) as Ref<RcloneFormConfig>
const showWebdavTip = ref(!localStorage.getItem('webdav_tip_dismissed'))
const commonFlags = [
  {
    category: 'Caching',
    flags: [
      { flag: '--vfs-cache-mode', value: 'full', descriptionKey: 'vfs-cache-mode-full' },
      { flag: '--vfs-cache-mode', value: 'writes', descriptionKey: 'vfs-cache-mode-writes' },
      { flag: '--vfs-cache-mode', value: 'minimal', descriptionKey: 'vfs-cache-mode-minimal' },
      { flag: '--vfs-cache-max-age', value: '24h', descriptionKey: 'vfs-cache-max-age' },
      { flag: '--vfs-cache-max-size', value: '10G', descriptionKey: 'vfs-cache-max-size' },
      { flag: '--dir-cache-time', value: '5m', descriptionKey: 'dir-cache-time' },
    ],
  },
  {
    category: 'Performance',
    flags: [
      { flag: '--buffer-size', value: '16M', descriptionKey: 'buffer-size-16M' },
      { flag: '--buffer-size', value: '32M', descriptionKey: 'buffer-size-32M' },
      { flag: '--vfs-read-chunk-size', value: '128M', descriptionKey: 'vfs-read-chunk-size' },
      { flag: '--transfers', value: '4', descriptionKey: 'transfers' },
      { flag: '--checkers', value: '8', descriptionKey: 'checkers' },
    ],
  },
  {
    category: 'Bandwidth',
    flags: [
      { flag: '--bwlimit', value: '10M', descriptionKey: 'bwlimit-10M' },
      { flag: '--bwlimit', value: '10M:100M', descriptionKey: 'bwlimit-10M:100M' },
      { flag: '--bwlimit', value: '08:00,512k 18:00,10M 23:00,off', descriptionKey: 'bwlimit-schedule' },
    ],
  },
  {
    category: 'Network',
    flags: [
      { flag: '--timeout', value: '5m', descriptionKey: 'timeout' },
      { flag: '--contimeout', value: '60s', descriptionKey: 'contimeout' },
      { flag: '--low-level-retries', value: '10', descriptionKey: 'low-level-retries' },
      { flag: '--retries', value: '3', descriptionKey: 'retries' },
    ],
  },
  {
    category: 'Security',
    flags: [
      { flag: '--read-only', value: '', descriptionKey: 'read-only' },
      { flag: '--allow-other', value: '', descriptionKey: 'allow-other' },
      { flag: '--allow-root', value: '', descriptionKey: 'allow-root' },
      { flag: '--umask', value: '022', descriptionKey: 'umask' },
    ],
  },
  {
    category: 'WebDAV Specific',
    flags: [
      { flag: '--webdav-headers', value: 'User-Agent,rclone/1.0', descriptionKey: 'webdav-headers' },
      { flag: '--webdav-bearer-token', value: '', descriptionKey: 'webdav-bearer-token' },
    ],
  },
  {
    category: 'Debugging',
    flags: [
      { flag: '--log-level', value: 'INFO', descriptionKey: 'log-level' },
      { flag: '--verbose', value: '', descriptionKey: 'verbose' },
      { flag: '--use-json-log', value: '', descriptionKey: 'use-json-log' },
      { flag: '--progress', value: '', descriptionKey: 'progress' },
    ],
  },
  {
    category: 'Windows Specific',
    flags: [
      { flag: '--volname', value: 'OpenlistDAV', descriptionKey: 'volname' },
    ],
  },
]

const filterList = computed(() => [
  { label: t('mount.filters.allStatus'), value: 'all' },
  { label: t('mount.status.mounted'), value: 'mounted' },
  { label: t('mount.status.unmounted'), value: 'unmounted' },
  { label: t('mount.status.error'), value: 'error' },
])

const statusMap = computed(() => {
  const map: Record<string, 'mounted' | 'unmounted' | 'error' | 'mounting'> = {}
  for (const mountInfo of appStore.mountInfos) {
    map[mountInfo.name] = mountInfo.status
  }
  return map
})

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
    error: appStore.mountInfos.filter(m => m.status === 'error').length,
  }
})

const addNewConfig = () => {
  resetForm()
  isAddingNew.value = true
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
    autoMount: config.autoMount || false,
    extraFlags: config.extraFlags || [],
  }
  showAddForm.value = true
}

const saveConfig = async () => {
  if (!configForm.value.name || !configForm.value.url || !configForm.value.user || !configForm.value.pass) {
    message.error(t('mount.messages.fillRequiredFields'))
    return
  }
  if (isConfigMounted(configForm.value.name)) {
    message.error(t('mount.messages.unmountBeforeEdit', { name: configForm.value.name }))
    return
  }

  try {
    if (editingConfig.value && editingConfig.value.name) {
      await appStore.updateRemoteConfig(editingConfig.value.name, configForm.value.type, {
        name: configForm.value.name,
        type: configForm.value.type,
        url: configForm.value.url,
        vendor: configForm.value.vendor || '',
        user: configForm.value.user,
        pass: configForm.value.pass,
        mountPoint: configForm.value.mountPoint || '',
        volumeName: configForm.value.volumeName || '',
        autoMount: configForm.value.autoMount,
        extraFlags: configForm.value.extraFlags,
      })
    } else {
      await appStore.createRemoteConfig(configForm.value.name, configForm.value.type, {
        name: configForm.value.name,
        type: configForm.value.type,
        url: configForm.value.url,
        vendor: configForm.value.vendor || '',
        user: configForm.value.user,
        pass: configForm.value.pass,
        mountPoint: configForm.value.mountPoint || '',
        volumeName: configForm.value.volumeName || '',
        autoMount: configForm.value.autoMount,
        extraFlags: configForm.value.extraFlags,
      })
    }
    showAddForm.value = false
    message.success(
      isAddingNew.value
        ? t('mount.messages.addedSuccessfully', { name: configForm.value.name })
        : t('mount.messages.updatedSuccessfully', { name: configForm.value.name }),
    )
    resetForm()
  } catch (error: any) {
    message.error(error.message || t('mount.messages.failedToSave'))
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
    extraFlags: [],
  }
  editingConfig.value = null
}

const mountConfig = async (config: RcloneFormConfig) => {
  try {
    loadingList.value.push(config.name)
    await appStore.mountRemote(config.name)
  } catch (error: any) {
    message.error(error.message || t('mount.messages.failedToMount'))
  } finally {
    const index = loadingList.value.indexOf(config.name)
    if (index !== -1) {
      loadingList.value.splice(index, 1)
    }
  }
}

const unmountConfig = async (config: RcloneFormConfig) => {
  if (!config.name) return
  try {
    await appStore.unmountRemote(config.name)
  } catch (error: any) {
    message.error(error.message || t('mount.messages.failedToUnmount'))
  }
}

const deleteConfig = async (config: RcloneFormConfig) => {
  if (!config.name) return
  const result = await confirm.confirm({
    message: t('mount.messages.confirmDelete', { name: config.name }),
    title: t('mount.messages.confirmDeleteTitle'),
    confirmButtonText: t('common.confirm'),
    cancelButtonText: t('common.cancel'),
    type: 'warning',
  })
  if (!result) {
    return
  }
  confirmDelete(config)
}

const confirmDelete = async (config: RcloneFormConfig) => {
  if (!config || !config.name) return

  try {
    await appStore.deleteRemoteConfig(config.name)
    message.success(t('mount.messages.deletedSuccessfully', { name: config.name }))
  } catch (error: any) {
    message.error(error.message || t('mount.messages.failedToDelete'))
  }
}

const refreshData = async () => {
  try {
    await appStore.loadRemoteConfigs()
    await appStore.loadMountInfos()
  } catch (error: any) {
    message.error(error.message)
  }
}

const getErrorMsg = (name: string) => {
  const mountInfo = appStore.mountInfos.find(m => m.name === name)
  return mountInfo?.error_msg || ''
}

const getConfigStatus = (name: string) => {
  return statusMap.value[name] || 'unmounted'
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

const isConfigMounted = (name: string) => {
  return getConfigStatus(name) === 'mounted'
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

const getFlagDescription = (flag: { flag: string; value: string; descriptionKey: string }) => {
  return t(`mount.config.flagDescriptions.${flag.descriptionKey}`)
}

const stopProcess = async (name: string) => {
  try {
    await appStore.unmountRemote(name)
    message.success(t('mount.messages.processStopped', { name }))
  } catch (error: any) {
    message.error(error.message || t('mount.messages.failedToStopProcess'))
  }
}

const openInFileExplorer = async (path?: string) => {
  if (!path) {
    message.error(t('mount.messages.mountPointPathNotAvailable'))
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

const dismissWebdavTip = () => {
  showWebdavTip.value = false
  localStorage.setItem('webdav_tip_dismissed', 'true')
}

const showWinfspTip = ref(isWindows && !localStorage.getItem('winfsp_tip_dismissed'))

const dismissWinfspTip = () => {
  showWinfspTip.value = false
  localStorage.setItem('winfsp_tip_dismissed', 'true')
}

const showRcloneTip = ref(false)

const dismissRcloneTip = () => {
  showRcloneTip.value = false
  localStorage.setItem('rclone_tip_dismissed', 'true')
}

const shouldShowWebdavTip = computed(() => {
  if (isWindows) {
    return !showWinfspTip.value && showWebdavTip.value
  }
  if (isLinux && showRcloneTip.value) {
    return false
  }
  return showWebdavTip.value
})

onMounted(async () => {
  initLoading.value = true
  appStore.loadRemoteConfigs()
  appStore.loadMountInfos()
  mountRefreshInterval = setInterval(appStore.loadMountInfos, 15 * 1000)
  rcloneStore.init()

  if (isLinux && !localStorage.getItem('rclone_tip_dismissed')) {
    const available = await rcloneStore.checkRcloneAvailable()
    showRcloneTip.value = !available
  }
  initLoading.value = false
})

onUnmounted(() => {
  if (mountRefreshInterval) {
    clearInterval(mountRefreshInterval)
  }
})
</script>
