// Windows, the tray icon, and the app:// protocol the windows load from.

import { BrowserWindow, Menu, Tray, app, nativeImage, nativeTheme, protocol, screen, shell } from 'electron'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { Downloads } from './downloads.js'
import { Hub, Settings } from './store.js'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const isMac = process.platform === 'darwin'
const isWindows = process.platform === 'win32'

/** The window colour the Mac app shows: matte black at 80% over the blurred desktop comes out as #2d2d2f, and
 * everything in the window is drawn relative to it. */
export const WINDOW_BACKGROUND = '#2d2d2f'
const TITLE_BAR_HEIGHT = 38

// MARK: - app:// protocol

export function registerScheme() {
    protocol.registerSchemesAsPrivileged([
        { scheme: 'app', privileges: { standard: true, secure: true, supportFetchAPI: true } },
    ])
}

const MIME = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.png': 'image/png',
    '.svg': 'image/svg+xml',
    '.woff2': 'font/woff2',
    '.json': 'application/json',
}

/** Maps app://purrloader/<path> to files in the app: the windows' code, assets, the font, and Preact. Preact's
 * hooks import "preact" by bare name, so that import is pointed at the served file. */
const ROUTES = [
    ['/vendor/preact.js', 'node_modules/preact/dist/preact.mjs'],
    ['/vendor/hooks.js', 'node_modules/preact/hooks/dist/hooks.mjs', (text) => text.replace(/from\s*["']preact["']/g, 'from"./preact.js"')],
    ['/vendor/htm.js', 'node_modules/htm/dist/htm.mjs'],
]

export function handleProtocol() {
    protocol.handle('app', async (request) => {
        const url = new URL(request.url)
        let pathname = decodeURIComponent(url.pathname)
        const route = ROUTES.find(([served]) => served === pathname)
        let file
        if (route) file = path.join(root, route[1])
        else if (pathname.startsWith('/fonts/inter/')) {
            file = path.join(root, 'node_modules/@fontsource-variable/inter', pathname.slice('/fonts/inter/'.length))
        } else if (pathname.startsWith('/assets/')) file = path.join(root, pathname)
        // The pure logic in src/core runs in the windows too, for labels and formatting.
        else if (pathname.startsWith('/core/')) file = path.join(root, 'src', pathname)
        else file = path.join(root, 'src/renderer', pathname)

        const allowed = ['src/renderer', 'src/core', 'assets', 'node_modules'].map((dir) => path.join(root, dir))
        const resolved = path.resolve(file)
        if (!allowed.some((dir) => resolved.startsWith(dir + path.sep))) return new Response('Not found', { status: 404 })
        try {
            let body = await fs.promises.readFile(resolved)
            if (route?.[2]) body = route[2](body.toString('utf8'))
            return new Response(body, {
                headers: { 'content-type': MIME[path.extname(resolved)] ?? 'application/octet-stream' },
            })
        } catch {
            return new Response('Not found', { status: 404 })
        }
    })
}

export function assetPath(name) {
    return path.join(root, 'assets', name)
}

// MARK: - Windows

const windows = { main: null, settings: null, history: null, tray: null }

function pageURL(view, params = {}) {
    const query = new URLSearchParams({ view, ...params })
    return `app://purrloader/index.html?${query}`
}

function makeWindow(view, options) {
    const window = new BrowserWindow({
        show: false,
        backgroundColor: WINDOW_BACKGROUND,
        ...(isMac ? {} : { icon: assetPath('icon.png') }),
        titleBarStyle: 'hidden',
        ...(isMac
            ? { trafficLightPosition: { x: 14, y: 13 } }
            : {
                  // The window buttons Windows and Linux draw themselves, over the same 38-point strip the Mac
                  // keeps for its traffic lights.
                  titleBarOverlay: { color: WINDOW_BACKGROUND, symbolColor: '#ffffffd9', height: TITLE_BAR_HEIGHT },
              }),
        webPreferences: {
            preload: path.join(root, 'src', 'preload.cjs'),
            contextIsolation: true,
            sandbox: true,
            nodeIntegration: false,
            spellcheck: false,
        },
        ...options,
    })
    window.removeMenu?.()
    window.loadURL(pageURL(view))
    // Snapshot runs (development only) keep their windows off the screen.
    window.once('ready-to-show', () => !process.env.PURRLOADER_SNAPSHOT && window.show())
    // Links open in the browser; nothing navigates the window itself.
    window.webContents.setWindowOpenHandler(({ url }) => {
        if (/^https?:\/\//.test(url)) shell.openExternal(url)
        return { action: 'deny' }
    })
    window.webContents.on('will-navigate', (event, url) => {
        if (!url.startsWith('app://')) event.preventDefault()
    })
    if (!app.isPackaged) {
        window.webContents.on('before-input-event', (event, input) => {
            if (input.type === 'keyDown' && input.key === 'I' && input.shift && (input.control || input.meta)) {
                window.webContents.toggleDevTools()
            }
        })
    }
    return window
}

export const Windows = {
    get main() {
        return windows.main
    },

    showMain() {
        if (!windows.main || windows.main.isDestroyed()) {
            windows.main = makeWindow('main', {
                title: 'Purrloader',
                width: 780,
                height: 660,
                minWidth: 720,
                minHeight: 580,
            })
            windows.main.on('close', (event) => {
                // Like the Mac app, Purrloader keeps running in the tray when its window closes.
                if (!app.isQuitting && TrayIcon.current) {
                    event.preventDefault()
                    windows.main.hide()
                } else if (!app.isQuitting) {
                    event.preventDefault()
                    app.quit()
                }
            })
            windows.main.on('focus', () => this.onMainFocus?.())
            windows.main.on('closed', () => (windows.main = null))
        } else if (!process.env.PURRLOADER_SNAPSHOT) {
            if (windows.main.isMinimized()) windows.main.restore()
            windows.main.show()
            windows.main.focus()
        }
        return windows.main
    },

    /** Settings resizes to fit each tab, like the Mac's Settings window. */
    showSettings(tab) {
        if (tab) Settings.set('settingsTab', tab)
        if (windows.settings && !windows.settings.isDestroyed()) {
            windows.settings.show()
            windows.settings.focus()
            return windows.settings
        }
        const size = SETTINGS_SIZES[Settings.get('settingsTab')] ?? SETTINGS_SIZES.general
        windows.settings = makeWindow('settings', {
            title: 'Settings',
            useContentSize: true,
            width: size[0],
            height: size[1],
            resizable: false,
            maximizable: false,
            minimizable: false,
            fullscreenable: false,
        })
        windows.settings.on('closed', () => (windows.settings = null))
        return windows.settings
    },

    resizeSettings(tab) {
        const window = windows.settings
        const size = SETTINGS_SIZES[tab]
        if (!window || window.isDestroyed() || !size) return
        window.setResizable(true)
        window.setContentSize(size[0], size[1], true)
        window.setResizable(false)
    },

    showHistory() {
        if (windows.history && !windows.history.isDestroyed()) {
            windows.history.show()
            windows.history.focus()
            return windows.history
        }
        windows.history = makeWindow('history', {
            title: 'History',
            width: 760,
            height: 560,
            minWidth: 560,
            minHeight: 360,
        })
        windows.history.on('closed', () => (windows.history = null))
        return windows.history
    },

    /** The panel the tray icon opens on Windows: the Mac app's menu bar window. */
    toggleTrayPanel(bounds) {
        if (windows.tray && !windows.tray.isDestroyed() && windows.tray.isVisible()) {
            windows.tray.hide()
            return
        }
        if (!windows.tray || windows.tray.isDestroyed()) {
            windows.tray = new BrowserWindow({
                show: false,
                frame: false,
                resizable: false,
                movable: false,
                minimizable: false,
                maximizable: false,
                fullscreenable: false,
                skipTaskbar: true,
                alwaysOnTop: true,
                useContentSize: true,
                width: 320,
                height: 120,
                backgroundColor: '#1e1e20',
                webPreferences: {
                    preload: path.join(root, 'src', 'preload.cjs'),
                    contextIsolation: true,
                    sandbox: true,
                    nodeIntegration: false,
                },
            })
            windows.tray.loadURL(pageURL('tray'))
            windows.tray.on('blur', () => windows.tray?.hide())
            windows.tray.on('closed', () => (windows.tray = null))
        }
        this.positionTrayPanel(bounds)
        windows.tray.show()
        windows.tray.focus()
    },

    positionTrayPanel(bounds) {
        const panel = windows.tray
        if (!panel || panel.isDestroyed()) return
        const [width, height] = panel.getSize()
        const anchor = bounds ?? screen.getCursorScreenPoint()
        const point = { x: anchor.x + (anchor.width ?? 0) / 2, y: anchor.y + (anchor.height ?? 0) / 2 }
        const area = screen.getDisplayNearestPoint(point).workArea
        let x = Math.round(point.x - width / 2)
        // The taskbar is usually at the bottom: open above it; if it's at the top, below it.
        let y = point.y > area.y + area.height / 2 ? area.y + area.height - height - 8 : area.y + 8
        x = Math.min(Math.max(x, area.x + 8), area.x + area.width - width - 8)
        panel.setPosition(x, Math.round(y))
    },

    fitTrayPanel(height) {
        const panel = windows.tray
        if (!panel || panel.isDestroyed()) return
        panel.setContentSize(320, Math.ceil(height))
        if (panel.isVisible()) this.positionTrayPanel(TrayIcon.current?.getBounds())
    },

    hideTrayPanel() {
        windows.tray?.hide()
    },

    all() {
        return Object.values(windows).filter((window) => window && !window.isDestroyed())
    },
}

const SETTINGS_SIZES = { general: [520, 352], advanced: [720, 672], about: [520, 652] }

// MARK: - Tray

let tray = null

/** The cat outline with its download arrow, white on dark taskbars and black on light ones. */
function trayImage() {
    const light = isWindows && nativeTheme.shouldUseLightColorsForSystemIntegratedUI
    const name = `tray-${light ? 'black' : 'white'}`
    const image = nativeImage.createFromPath(assetPath(`${name}.png`))
    for (const scale of [1.5, 2]) {
        const file = assetPath(`${name}@${scale}x.png`)
        if (fs.existsSync(file)) image.addRepresentation({ scaleFactor: scale, buffer: fs.readFileSync(file) })
    }
    return image
}

export const TrayIcon = {
    menuTimer: null,

    get current() {
        return tray
    },

    init() {
        nativeTheme.on('updated', () => tray?.setImage(trayImage()))
        this.update()
        Hub.listeners.add((patch) => {
            if (patch.jobs) this.refresh()
            if (patch.settings && Boolean(tray) !== this.wanted) this.update()
        })
    },

    get wanted() {
        return Boolean(Settings.get('showTrayIcon')) && !isMac
    },

    /** Shows or hides the icon to match "Show Purrloader in the notification area". */
    update() {
        if (this.wanted && !tray) {
            tray = new Tray(trayImage())
            tray.setToolTip('Purrloader')
            if (isWindows) {
                tray.on('click', (_, bounds) => Windows.toggleTrayPanel(bounds))
                tray.on('right-click', () => tray.popUpContextMenu(this.menu()))
            } else {
                // Linux tray icons can only show a menu.
                tray.setContextMenu(this.menu())
            }
        } else if (!this.wanted && tray) {
            tray.destroy()
            tray = null
            Windows.hideTrayPanel()
        }
        this.refresh()
    },

    /** Progress in the tooltip and on the taskbar button. Linux menus are rebuilt at most once a second. */
    refresh() {
        const active = Downloads.activeCount
        const queued = Downloads.queuedCount
        const fraction = Downloads.overallFraction
        const main = Windows.main
        if (main && !main.isDestroyed()) main.setProgressBar(fraction ?? (active > 0 ? 2 : -1))
        if (!isWindows) app.setBadgeCount(active + queued)
        if (!tray) return
        const status = fraction != null ? `${Math.round(fraction * 100)}%` : `${active} active`
        tray.setToolTip(active > 0 ? `Purrloader · ${status}` : 'Purrloader')
        if (!isWindows && !this.menuTimer) {
            this.menuTimer = setTimeout(() => {
                this.menuTimer = null
                tray?.setContextMenu(this.menu())
            }, 1000)
        }
    },

    menu() {
        const active = Downloads.activeCount
        const queued = Downloads.queuedCount
        const summary =
            active === 0 && queued === 0 ? 'Idle' : queued > 0 ? `${active} active · ${queued} waiting` : `${active} active`
        const items = [{ label: `Purrloader — ${summary}`, enabled: false }]
        const visible = Downloads.jobs.filter((job) => !['finished', 'cancelled', 'failed'].includes(job.status)).slice(0, 6)
        if (visible.length) {
            items.push({ type: 'separator' })
            for (const job of visible) {
                const percent = job.fraction != null ? ` · ${Math.round(job.fraction * 100)}%` : ''
                items.push({ label: truncate(job.title, 48) + percent, enabled: false })
            }
        }
        items.push({ type: 'separator' }, { label: 'Open Purrloader', click: () => Windows.showMain() })
        if (active > 0 || queued > 0) items.push({ label: 'Pause All', click: () => Downloads.pauseAll() })
        else if (Downloads.jobs.some((job) => job.status === 'paused')) {
            items.push({ label: 'Resume All', click: () => Downloads.resumeAll() })
        }
        items.push({ type: 'separator' }, { label: 'Quit Purrloader', click: () => app.quit() })
        return Menu.buildFromTemplate(items)
    },
}

function truncate(text, length) {
    return text.length > length ? text.slice(0, length - 1) + '…' : text
}
