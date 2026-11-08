// Browsers whose sign-in yt-dlp can read, limited to the ones installed on this computer. yt-dlp reads each
// browser's profile folder, so a browser counts as installed when that folder exists.

import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const home = os.homedir()
const env = process.env
const local = env.LOCALAPPDATA ?? path.join(home, 'AppData', 'Local')
const roaming = env.APPDATA ?? path.join(home, 'AppData', 'Roaming')
const config = env.XDG_CONFIG_HOME ?? path.join(home, '.config')

const PROFILE_FOLDERS = {
    win32: {
        chrome: [path.join(local, 'Google', 'Chrome', 'User Data')],
        firefox: [path.join(roaming, 'Mozilla', 'Firefox', 'Profiles')],
        brave: [path.join(local, 'BraveSoftware', 'Brave-Browser', 'User Data')],
        edge: [path.join(local, 'Microsoft', 'Edge', 'User Data')],
        chromium: [path.join(local, 'Chromium', 'User Data')],
        vivaldi: [path.join(local, 'Vivaldi', 'User Data')],
        opera: [path.join(roaming, 'Opera Software', 'Opera Stable')],
    },
    linux: {
        chrome: [path.join(config, 'google-chrome')],
        firefox: [
            path.join(home, '.mozilla', 'firefox'),
            path.join(home, 'snap', 'firefox', 'common', '.mozilla', 'firefox'),
            path.join(home, '.var', 'app', 'org.mozilla.firefox', '.mozilla', 'firefox'),
        ],
        brave: [path.join(config, 'BraveSoftware', 'Brave-Browser')],
        edge: [path.join(config, 'microsoft-edge')],
        chromium: [path.join(config, 'chromium'), path.join(home, 'snap', 'chromium', 'common', 'chromium')],
        vivaldi: [path.join(config, 'vivaldi')],
        opera: [path.join(config, 'opera')],
    },
    darwin: {
        safari: ['/Applications/Safari.app'],
        chrome: ['/Applications/Google Chrome.app'],
        firefox: ['/Applications/Firefox.app'],
        arc: ['/Applications/Arc.app'],
        brave: ['/Applications/Brave Browser.app'],
        edge: ['/Applications/Microsoft Edge.app'],
        chromium: ['/Applications/Chromium.app'],
        vivaldi: ['/Applications/Vivaldi.app'],
        opera: ['/Applications/Opera.app'],
    },
}

/** Every browser yt-dlp can read on this platform, in the Mac app's order. */
export const SUPPORTED_BROWSERS = Object.keys(PROFILE_FOLDERS[process.platform] ?? PROFILE_FOLDERS.linux)

let installed = null

export function installedBrowsers() {
    if (installed) return installed
    const folders = PROFILE_FOLDERS[process.platform] ?? PROFILE_FOLDERS.linux
    const found = Object.entries(folders)
        .filter(([, candidates]) => candidates.some((folder) => fs.existsSync(folder)))
        .map(([browser]) => browser)
    installed = found.length ? found : SUPPORTED_BROWSERS
    return installed
}

/** The note under the browser list: what each platform needs for yt-dlp to read a sign-in. */
export function browserNote() {
    switch (process.platform) {
        case 'win32':
            return 'Firefox works best. Chrome and Edge keep their sign-in locked on Windows, so close them first, and they may still refuse.'
        case 'linux':
            return 'Chrome, Edge and other Chromium browsers may ask for your keyring password.'
        default:
            return 'Safari needs Full Disk Access for Purrloader in System Settings → Privacy & Security.'
    }
}
