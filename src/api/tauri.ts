import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { appDataDir, join } from '@tauri-apps/api/path'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater'

let pendingUpdateInstance: Awaited<ReturnType<typeof check>> = null

export class TauriAPI {
  // --- OpenList Core management ---
  static core = {
    start: (): Promise<ProcessInfo> => invoke('start_openlist_core'),
    stop: (): Promise<ProcessInfo> => invoke('stop_openlist_core'),
    getStatus: (): Promise<OpenListCoreStatus> => invoke('get_openlist_core_status'),
  }

  // --- Rclone management ---
  static rclone = {
    // Check if rclone binary is available
    isAvailable: (): Promise<boolean> => invoke('check_rclone_available'),

    // Remote configuration management (direct file-based)
    remotes: {
      list: (): Promise<string[]> => invoke('rclone_list_remotes'),
      create: (name: string, type: string, config: RcloneWebdavConfig): Promise<boolean> =>
        invoke('rclone_create_remote', { name, type, config }),
      update: (name: string, type: string, config: RcloneWebdavConfig): Promise<boolean> =>
        invoke('rclone_update_remote', { name, type, config }),
      delete: (name: string): Promise<boolean> => invoke('rclone_delete_remote', { name }),
      listConfig: (t: string): Promise<IRemoteConfig> => invoke('rclone_list_config', { remoteType: t }),
    },

    // Mount process management
    mounts: {
      list: (): Promise<RcloneMountInfo[]> => invoke('get_mount_info_list'),
      check: (mp: string): Promise<boolean> => invoke('check_mount_status', { mountPoint: mp }),
      mount: (cfg: MountProcessInput): Promise<ProcessInfo> => invoke('mount_remote', { config: cfg }),
      unmount: (name: string): Promise<boolean> => invoke('unmount_remote', { name }),
    },
  }

  // -- File management ---
  static files = {
    open: (path: string): Promise<boolean> => invoke('open_file', { path }),
    folder: (path: string): Promise<boolean> => invoke('open_folder', { path }),
    urlInBrowser: (url: string): Promise<boolean> => invoke('open_url_in_browser', { url }),
    openOpenListDataDir: (): Promise<boolean> => invoke('open_openlist_data_dir'),
    openLogsDirectory: (): Promise<boolean> => invoke('open_logs_directory'),
    openRcloneConfigFile: (): Promise<boolean> => invoke('open_rclone_config_file'),
    openSettingsFile: (): Promise<boolean> => invoke('open_settings_file'),
  }

  // --- Settings management ---
  static settings = {
    load: (): Promise<MergedSettings | null> => invoke('load_settings'),
    save: (s: MergedSettings): Promise<boolean> => invoke('save_settings', { settings: s }),
    saveAndRestart: (s: MergedSettings): Promise<boolean> => invoke('save_settings_and_restart', { settings: s }),
    reset: (): Promise<MergedSettings | null> => invoke('reset_settings'),
  }

  // --- Logs management ---
  static logs = {
    get: (src?: 'openlist' | 'rclone' | 'app' | 'openlist_core' | 'all'): Promise<string[]> =>
      invoke('get_logs', { source: src }),
    clear: (src?: 'openlist' | 'rclone' | 'app' | 'openlist_core' | 'all'): Promise<boolean> =>
      invoke('clear_logs', { source: src }),
    adminPassword: (): Promise<string> => invoke('get_admin_password'),
    resetAdminPassword: (): Promise<string> => invoke('reset_admin_password'),
    setAdminPassword: (password: string): Promise<string> => invoke('set_admin_password', { password }),
  }

  // --- Binary management ---
  static bin = {
    version: (name: 'openlist' | 'rclone'): Promise<string> => invoke('get_binary_version', { binaryName: name }),
    availableVersions: (tool: string, force: boolean): Promise<string[]> =>
      invoke('get_available_versions', { tool, force }),
    updateVersion: (tool: string, version: string): Promise<string> => invoke('update_tool_version', { tool, version }),
  }

  // --- Utility functions ---
  static util = {
    defaultDataDir: (): Promise<string> => appDataDir().then(d => join(d, 'openlist-desktop')),
    defaultConfig: (): Promise<string> => appDataDir().then(d => join(d, 'openlist-desktop', 'rclone.conf')),
    selectDirectory: (title: string): Promise<string | null> => invoke('select_directory', { title }),
  }

  // --- Tray management ---
  static tray = {
    update: (r: boolean): Promise<void> => invoke('update_tray_menu', { serviceRunning: r }),
  }

  // --- macOS Dock management ---
  static dock = {
    setVisibility: (visible: boolean): Promise<boolean> => invoke('set_dock_icon_visibility', { visible }),
  }

  // --- Firewall management ---
  static firewall = {
    check: (): Promise<boolean> => invoke('check_firewall_rule'),
    add: (): Promise<boolean> => invoke('add_firewall_rule'),
    remove: (): Promise<boolean> => invoke('remove_firewall_rule'),
  }

  // --- Update management ---
  static updater = {
    check: async (): Promise<UpdateCheck> => {
      const update = await check()
      const currentVersion = await invoke<string>('get_current_version')

      pendingUpdateInstance = update

      if (update) {
        return {
          hasUpdate: true,
          currentVersion,
          latestVersion: update.version,
          releaseDate: update.date ?? '',
          releaseNotes: update.body ?? '',
          assets: [],
        }
      }

      return {
        hasUpdate: false,
        currentVersion,
        latestVersion: currentVersion,
        releaseDate: '',
        releaseNotes: '',
        assets: [],
      }
    },

    hasPendingUpdate: (): boolean => {
      return pendingUpdateInstance !== null
    },

    downloadAndInstall: async (onProgress?: (progress: DownloadProgress) => void): Promise<void> => {
      if (!pendingUpdateInstance) {
        throw new Error('No pending update available. Call check() first.')
      }

      let downloaded = 0
      let contentLength = 0

      await pendingUpdateInstance.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0
            if (onProgress) {
              onProgress({
                downloaded: 0,
                total: contentLength,
                percentage: 0,
                speed: 0,
              })
            }
            break
          case 'Progress':
            downloaded += event.data.chunkLength
            if (onProgress) {
              onProgress({
                downloaded,
                total: contentLength,
                percentage: contentLength > 0 ? (downloaded / contentLength) * 100 : 0,
                speed: event.data.chunkLength, // Approximate speed
              })
            }
            break
          case 'Finished':
            if (onProgress) {
              onProgress({
                downloaded: contentLength,
                total: contentLength,
                percentage: 100,
                speed: 0,
              })
            }
            break
        }
      })

      pendingUpdateInstance = null
    },

    clearPendingUpdate: (): void => {
      pendingUpdateInstance = null
    },

    relaunch: async (): Promise<void> => {
      await relaunch()
    },

    currentVersion: (): Promise<string> => invoke('get_current_version'),
    setAutoCheck: (e: boolean): Promise<void> => invoke('set_auto_check_enabled', { enabled: e }),
    isAutoCheckEnabled: (): Promise<boolean> => invoke('is_auto_check_enabled'),
    onBackgroundUpdate: (cb: (u: UpdateCheck) => void) =>
      listen('background-update-available', e => cb(e.payload as UpdateCheck)),
  }
}
