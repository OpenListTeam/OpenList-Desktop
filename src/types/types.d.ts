declare const OS_PLATFORM: Platform

type IRemoteConfig = Record<string, RcloneWebdavConfig>

interface OpenListCoreConfig {
  port: number
  data_dir: string
  auto_launch: boolean
  ssl_enabled: boolean
  binary_path?: string
}

interface RcloneConfig {
  mount_config: Record<string, RcloneFormConfig>
  binary_path?: string
  rclone_conf_path?: string
}

interface RcloneWebdavConfig {
  url: string
  vendor?: string
  user: string
  pass: string
}

interface RcloneFormConfig {
  name: string
  type: string
  url: string
  vendor?: string
  user: string
  pass: string
  mountPoint?: string
  volumeName?: string
  extraFlags?: string[]
  autoMount: boolean
  networkMode: boolean
}

interface RcloneMountInfo {
  name: string
  processId: string
  remotePath: string
  mountPoint: string
  status: 'mounted' | 'unmounted' | 'error'
  error_msg?: string
}

interface AppConfig {
  theme?: 'light' | 'dark' | 'auto'
  auto_update_enabled?: boolean
  gh_proxy?: string
  gh_proxy_api?: boolean
  open_links_in_browser?: boolean
  admin_password?: string
  show_window_on_startup?: boolean
  log_filter_level?: string
  log_filter_source?: string
  hide_dock_icon?: boolean
}

interface MergedSettings {
  openlist: OpenListCoreConfig
  rclone: RcloneConfig
  app: AppConfig
}

interface OpenListCoreStatus {
  running: boolean
  pid?: number
  port?: number
}

// ProcessConfig for creating/registering processes
interface ProcessConfig {
  id: string
  name: string
  bin_path: string
  args: string[]
  log_file: string
  working_dir?: string
  env_vars?: Record<string, string>
}

// ProcessInfo returned from process manager operations
interface ProcessInfo {
  id: string
  name: string
  is_running: boolean
  pid?: number
  started_at?: number
  config: ProcessConfig
}

// Input for creating mount processes
interface MountProcessInput {
  id: string
  name: string
  args: string[]
  networkMode: boolean
  auto_start?: boolean
}

interface UpdateAsset {
  name: string
  url: string
  size: number
  platform: string
  type: 'exe' | 'deb' | 'rpm' | 'dmg' | string
}

interface UpdateCheck {
  hasUpdate: boolean
  currentVersion: string
  latestVersion: string
  releaseDate: string
  releaseNotes: string
  assets: UpdateAsset[]
}

interface DownloadProgress {
  downloaded: number
  total: number
  percentage: number
  speed: number
}
