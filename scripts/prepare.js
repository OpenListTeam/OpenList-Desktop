import { execSync } from 'node:child_process'
import crypto from 'node:crypto'
import fsp from 'node:fs/promises'
import path from 'node:path'

import AdmZip from 'adm-zip'
import fs from 'fs-extra'
import fetch from 'node-fetch'
import { extract } from 'tar'

import { downloadFile, getFetchOptions, getOpenlistArchMap, getRcloneArchMap, platformIdentifierMap } from './utils.js'

const cwd = process.cwd()
const target = process.argv.find(arg => arg.startsWith('--target='))?.slice(9)
const platformArch = target ? platformIdentifierMap[target] : `${process.platform}-${process.arch}`
const [platform, arch] = platformArch.split('-')
const isWin = platform === 'win32'

console.log(`platform: ${platform}, arch: ${arch}`)

const sidecarHost =
  target ||
  execSync('rustc -vV')
    .toString()
    .match(/(?<=host: ).+(?=\s*)/g)[0]

if (!getOpenlistArchMap[platformArch]) {
  throw new Error(`Unsupported target: ${platformArch}. Supported: ${Object.keys(getOpenlistArchMap).join(', ')}`)
}

// Rclone version management
let rcloneVersion = 'v1.70.3'
const rcloneVersionUrl = 'https://github.com/rclone/rclone/releases/latest/download/version.txt'

async function getLatestRcloneVersion() {
  try {
    const response = await fetch(rcloneVersionUrl, getFetchOptions())
    rcloneVersion = (await response.text()).trim().replace('rclone ', '') || '1.70.1'
    console.log(`Latest rclone version: ${rcloneVersion}`)
  } catch (error) {
    console.log('Error fetching latest rclone version:', error.message)
  }
}

// openlist version management
let openlistVersion = 'v4.0.8'

async function getLatestOpenlistVersion() {
  try {
    const response = await fetch(
      'https://api.github.com/repos/OpenListTeam/OpenList/releases/latest',
      getFetchOptions()
    )
    const data = await response.json()
    openlistVersion = data.tag_name || 'v4.0.8'
    console.log(`Latest OpenList version: ${openlistVersion}`)
  } catch (error) {
    console.log('Error fetching latest OpenList version:', error.message)
  }
}

const createBinaryInfo = (name, archMap, baseUrl, version = '') => {
  const zipName = archMap[platformArch]
  return {
    name,
    targetFile: `${name}-${sidecarHost}${isWin ? '.exe' : ''}`,
    exeFile: `${name}${isWin ? '.exe' : ''}`,
    zipFile: zipName,
    downloadURL: `${baseUrl}${version ? `/${version}` : ''}/${zipName}`
  }
}

// OpenList service URL
const serviceUrl = `https://github.com/OpenListTeam/openlist-desktop-service/releases/download/${sidecarHost}`

const getServiceInfo = exeName => {
  const ext = isWin ? '.exe' : ''
  const suffix = '-' + sidecarHost
  return {
    file: exeName + suffix + ext,
    downloadURL: `${serviceUrl}/${exeName}${ext}`
  }
}

const resolvePlugins = async () => {
  const pluginDir = path.join(process.env.APPDATA, 'Local/NSIS')
  await fs.mkdir(pluginDir, { recursive: true })
  await resolveSimpleServicePlugin(pluginDir)
  await resolveAccessControlPlugin(pluginDir)
}

const resolveSimpleServicePlugin = async pluginDir => {
  const url = 'https://nsis.sourceforge.io/mediawiki/images/e/ef/NSIS_Simple_Service_Plugin_Unicode_1.30.zip'
  const TEMP_DIR = path.join(cwd, 'temp')
  const tempDir = path.join(TEMP_DIR, 'SimpleSC')
  const tempZip = path.join(tempDir, 'NSIS_Simple_Service_Plugin_Unicode_1.30.zip')
  const tempDll = path.join(tempDir, 'SimpleSC.dll')
  const pluginPath = path.join(pluginDir, 'SimpleSC.dll')

  await fs.mkdir(tempDir, { recursive: true })
  if (fs.existsSync(pluginPath)) return

  try {
    if (!fs.existsSync(tempZip)) {
      await downloadFile(url, tempZip)
    }
    const zip = new AdmZip(tempZip)
    zip.extractAllTo(tempDir, true)
    await fsp.cp(tempDll, pluginPath, { recursive: true, force: true })
    console.log(`SimpleSC.dll copied to ${pluginPath}`)
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

const calculateSha256 = async filePath => {
  const hash = crypto.createHash('sha256')
  const fileStream = fs.createReadStream(filePath)
  fileStream.on('data', chunk => hash.update(chunk))
  fileStream.on('end', () => {
    const digest = hash.digest('hex')
    console.log(`SHA-256 hash of ${filePath}: ${digest}`)
  })
}

const resolveAccessControlPlugin = async pluginDir => {
  const url = 'https://nsis.sourceforge.io/mediawiki/images/4/4a/AccessControl.zip'
  const TEMP_DIR = path.join(cwd, 'temp')
  const tempDir = path.join(TEMP_DIR, 'AccessControl')
  const tempZip = path.join(tempDir, 'AccessControl.zip')
  const tempDll = path.join(tempDir, 'Plugins', 'AccessControl.dll')
  const pluginPath = path.join(pluginDir, 'Plugins', 'x86-unicode', 'AccessControl.dll')
  const pluginPathB = path.join(pluginDir, 'AccessControl.dll')

  await fs.mkdir(tempDir, { recursive: true })
  if (fs.existsSync(pluginPath)) return

  try {
    if (!fs.existsSync(tempZip)) {
      await downloadFile(url, tempZip)
    }
    const zip = new AdmZip(tempZip)
    zip.extractAllTo(tempDir, true)

    let sourcePath = tempDll
    if (!fs.existsSync(sourcePath)) {
      const altPaths = [
        path.join(tempDir, 'AccessControl.dll'),
        path.join(tempDir, 'Plugins', 'i386-unicode', 'AccessControl.dll')
      ]
      for (const altPath of altPaths) {
        if (fs.existsSync(altPath)) {
          sourcePath = altPath
          break
        }
      }
    }
    if (fs.existsSync(sourcePath)) {
      await fsp.cp(sourcePath, pluginPath, { recursive: true, force: true })
      await fsp.cp(sourcePath, pluginPathB, { recursive: true, force: true })
      console.log(`AccessControl.dll copied to ${pluginPath}`)
    } else {
      console.warn('AccessControl.dll not found in the extracted archive')
    }
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

async function resolveSidecar(binInfo) {
  const { name, targetFile, zipFile, exeFile, downloadURL } = binInfo
  const binaryDir = path.join(cwd, 'src-tauri', 'binary')
  const binaryPath = path.join(binaryDir, targetFile)

  await fs.mkdir(binaryDir, { recursive: true })
  const zipPath = path.join(binaryDir, zipFile)

  try {
    await downloadFile(downloadURL, zipPath)

    if (zipFile.endsWith('.zip')) {
      const zip = new AdmZip(zipPath)
      zip.extractAllTo(binaryDir, true)

      if (name === 'rclone') {
        const extractedDir = path.join(binaryDir, zipFile.replace('.zip', ''))
        await fs.rename(path.join(extractedDir, exeFile), binaryPath)
        await fs.remove(extractedDir)
      } else {
        await fs.rename(path.join(binaryDir, exeFile), binaryPath)
      }
    } else {
      await extract({ cwd: binaryDir, file: zipPath })
      await fs.rename(path.join(binaryDir, exeFile), binaryPath)
    }
    await fs.remove(zipPath)
    await fs.chmod(binaryPath, 0o755)
    await calculateSha256(binaryPath)
  } catch (err) {
    console.error(`Error preparing "${name}":`, err.message)
    await fs.rm(binaryPath, { recursive: true, force: true })
    throw err
  }
}

async function resolveService(resourceInfo, isChmod = true, defaultMode = 0o755) {
  const { file, downloadURL } = resourceInfo
  const resourceDir = path.join(cwd, 'src-tauri', 'binary')
  const resourcePath = path.join(resourceDir, file)

  await fs.mkdir(resourceDir, { recursive: true })

  try {
    await downloadFile(downloadURL, resourcePath)
    if (isChmod) {
      await fs.chmod(resourcePath, defaultMode)
    }
    console.log(`"${file}" downloaded to ${resourcePath}`)
  } catch (err) {
    console.error(`Error preparing "${file}":`, err.message)
    await fs.rm(resourcePath, { recursive: true, force: true })
    throw err
  }
}

async function retryTask(name, fn, maxRetries = 5) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      console.log(`task::${name} try ${i}`)
      await fn()
      return
    } catch (err) {
      console.log(`task::${name} try ${i} ==`, err.message)
      if (i === maxRetries - 1) throw err
    }
  }
}

async function main() {
  await getLatestOpenlistVersion()
  await retryTask('openlist', () =>
    resolveSidecar(
      createBinaryInfo(
        'openlist',
        getOpenlistArchMap,
        'https://github.com/OpenListTeam/OpenList/releases/download',
        openlistVersion
      )
    )
  )

  // Only bundle rclone for Windows and macOS, Linux users should install rclone from system package manager
  const isLinux = platform === 'linux'
  if (!isLinux) {
    await retryTask('rclone', async () => {
      await getLatestRcloneVersion()
      await resolveSidecar(
        createBinaryInfo('rclone', getRcloneArchMap(rcloneVersion), `https://downloads.rclone.org`, rcloneVersion)
      )
    })
  } else {
    console.log('Skipping rclone download for Linux - users should install rclone from system package manager')
  }

  if (isWin) {
    await resolvePlugins()
  }
  await resolveService(getServiceInfo('install-openlist-service'))
  await resolveService(getServiceInfo('openlist-desktop-service'))
  await resolveService(getServiceInfo('uninstall-openlist-service'))
}

main().catch(console.log)
