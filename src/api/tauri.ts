import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { appDataDir, join } from '@tauri-apps/api/path'

const call = <T>(cmd: string, args?: any): Promise<T> => invoke(cmd, args)

export class TauriAPI {
  // --- service management ---
  static service = {
    status: (): Promise<string> => call('check_service_status'),
    install: (): Promise<boolean> => call('install_service'),
    uninstall: (): Promise<boolean> => call('uninstall_service'),
    start: (): Promise<boolean> => call('start_service'),
    stop: (): Promise<boolean> => call('stop_service')
  }

  // --- process management ---
  static process = {
    list: (): Promise<ProcessStatus[]> => call('get_process_list'),
    start: (id: string): Promise<boolean> => call('start_process', { id }),
    stop: (id: string): Promise<boolean> => call('stop_process', { id }),
    restart: (id: string): Promise<boolean> => call('restart_process', { id }),
    update: (id: string, cfg: Partial<ProcessConfig>): Promise<boolean> =>
      call('update_process', { id, updateConfig: cfg }),
    delete: (id: string): Promise<boolean> => call('delete_process', { id })
  }

  // --- OpenList Core management ---
  static core = {
    create: (autoStart: boolean): Promise<ProcessConfig> => call('create_openlist_core_process', { autoStart }),
    getStatus: (): Promise<OpenListCoreStatus> => call('get_openlist_core_status')
  }

  // --- Rclone management ---
  static rclone = {
    backend: {
      create: (): Promise<boolean> => call('create_rclone_backend_process'),
      createAndStart: (): Promise<ProcessConfig> => call('create_and_start_rclone_backend'),
      isRunning: (): Promise<boolean> => call('get_rclone_backend_status'),
      isAvailable: (): Promise<boolean> => call('check_rclone_available')
    },
    remotes: {
      list: (): Promise<string[]> => call('rclone_list_remotes'),
      create: (name: string, type: string, config: RcloneWebdavConfig): Promise<boolean> =>
        call('rclone_create_remote', { name, type, config }),
      update: (name: string, type: string, config: RcloneWebdavConfig): Promise<boolean> =>
        call('rclone_update_remote', { name, type, config }),
      delete: (name: string): Promise<boolean> => call('rclone_delete_remote', { name }),
      listConfig: (t: string): Promise<IRemoteConfig> => call('rclone_list_config', { remoteType: t })
    },
    mounts: {
      list: (): Promise<RcloneMountInfo[]> => call('get_mount_info_list'),
      check: (mp: string): Promise<boolean> => call('check_mount_status', { mountPoint: mp }),
      createProcess: (cfg: ProcessConfig): Promise<ProcessConfig> =>
        call('create_rclone_mount_remote_process', { config: cfg })
    }
  }

  // -- File management ---
  static files = {
    list: (path: string): Promise<FileItem[]> => call('list_files', { path }),
    open: (path: string): Promise<boolean> => call('open_file', { path }),
    folder: (path: string): Promise<boolean> => call('open_folder', { path }),
    url: (url: string): Promise<boolean> => call('open_url', { url }),
    urlInBrowser: (url: string): Promise<boolean> => call('open_url_in_browser', { url }),
    openOpenListDataDir: (): Promise<boolean> => call('open_openlist_data_dir'),
    openLogsDirectory: (): Promise<boolean> => call('open_logs_directory'),
    openRcloneConfigFile: (): Promise<boolean> => call('open_rclone_config_file'),
    openSettingsFile: (): Promise<boolean> => call('open_settings_file')
  }

  // --- Settings management ---
  static settings = {
    load: (): Promise<MergedSettings | null> => call('load_settings'),
    save: (s: MergedSettings): Promise<boolean> => call('save_settings', { settings: s }),
    saveWithUpdatePort: (s: MergedSettings): Promise<boolean> =>
      call('save_settings_with_update_port', { settings: s }),
    reset: (): Promise<MergedSettings | null> => call('reset_settings')
  }

  // --- Logs management ---
  static logs = {
    get: (src?: 'openlist' | 'rclone' | 'app' | 'openlist_core' | 'service' | 'all'): Promise<string[]> =>
      call('get_logs', { source: src }),
    clear: (src?: 'openlist' | 'rclone' | 'app' | 'openlist_core' | 'service' | 'all'): Promise<boolean> =>
      call('clear_logs', { source: src }),
    adminPassword: (): Promise<string> => call('get_admin_password'),
    resetAdminPassword: (): Promise<string> => call('reset_admin_password'),
    setAdminPassword: (password: string): Promise<string> => call('set_admin_password', { password })
  }

  // --- Binary management ---
  static bin = {
    version: (name: 'openlist' | 'rclone'): Promise<string> => call('get_binary_version', { binaryName: name }),
    availableVersions: (tool: string): Promise<string[]> => call('get_available_versions', { tool }),
    updateVersion: (tool: string, version: string): Promise<string> => call('update_tool_version', { tool, version })
  }

  // --- Utility functions ---
  static util = {
    defaultDataDir: (): Promise<string> => appDataDir().then(d => join(d, 'openlist-desktop')),
    defaultConfig: (): Promise<string> => appDataDir().then(d => join(d, 'openlist-desktop', 'rclone.conf')),
    selectDirectory: (title: string): Promise<string | null> => call('select_directory', { title })
  }

  // --- Tray management ---
  static tray = {
    update: (r: boolean): Promise<void> => call('update_tray_menu', { serviceRunning: r }),
    updateDelayed: (r: boolean): Promise<void> => call('update_tray_menu_delayed', { serviceRunning: r }),
    forceUpdate: (r: boolean): Promise<void> => call('force_update_tray_menu', { serviceRunning: r }),
    listen: (cb: (action: string) => void) => listen('tray-core-action', e => cb(e.payload as string))
  }

  // --- Firewall management ---
  static firewall = {
    check: (): Promise<boolean> => call('check_firewall_rule'),
    add: (): Promise<boolean> => call('add_firewall_rule'),
    remove: (): Promise<boolean> => call('remove_firewall_rule')
  }

  // --- Update management ---
  static updater = {
    check: (): Promise<UpdateCheck> => call('check_for_updates'),
    download: (url: string, name: string): Promise<string> =>
      call('download_update', { assetUrl: url, assetName: name }),
    installAndRestart: (path: string): Promise<void> => call('install_update_and_restart', { installerPath: path }),
    currentVersion: (): Promise<string> => call('get_current_version'),
    setAutoCheck: (e: boolean): Promise<void> => call('set_auto_check_enabled', { enabled: e }),
    isAutoCheckEnabled: (): Promise<boolean> => call('is_auto_check_enabled'),
    onBackgroundUpdate: (cb: (u: UpdateCheck) => void) =>
      listen('background-update-available', e => cb(e.payload as UpdateCheck)),
    onDownloadProgress: (cb: (p: DownloadProgress) => void) =>
      listen('download-progress', e => cb(e.payload as DownloadProgress)),
    onInstallStarted: (cb: () => void) => listen('update-install-started', () => cb()),
    onInstallError: (cb: (err: string) => void) => listen('update-install-error', e => cb(e.payload as string)),
    onAppQuit: (cb: () => void) => listen('quit-app', () => cb())
  }
}
