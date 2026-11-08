// Where Purrloader keeps its files, versioned JSON files, the settings store, and the hub that sends state to
// the windows. Together these stand in for the Mac app's AppPaths, JSONFile, UserDefaults and @Observable.

import { BrowserWindow, app } from 'electron'
import fs from 'node:fs'
import path from 'node:path'

function ensureDir(dir) {
    fs.mkdirSync(dir, { recursive: true })
    return dir
}

/** Everything lives in the per-user app data folder: %APPDATA%\Purrloader on Windows, ~/.config/Purrloader on
 * Linux. Chromium's own caches go into a subfolder so they don't mix with Purrloader's files. */
export const AppPaths = {
    init() {
        const support = ensureDir(app.getPath('userData'))
        this.support = support
        this.bin = ensureDir(path.join(support, 'bin'))
        this.temp = ensureDir(path.join(support, 'tmp'))
        this.archive = path.join(support, 'download-archive.txt')
        this.presets = path.join(support, 'presets.json')
        this.history = path.join(support, 'history.json')
        this.queue = path.join(support, 'queue.json')
        this.settings = path.join(support, 'settings.json')
    },
    get defaultDownloads() {
        return app.getPath('downloads')
    },
}

/** JSON files with a schemaVersion, so later versions of Purrloader know what they're reading. */
export const JSONFile = {
    /** null if the file is missing, unreadable, or written by a newer schema. */
    load(file, schemaVersion) {
        try {
            const envelope = JSON.parse(fs.readFileSync(file, 'utf8'))
            if (typeof envelope?.schemaVersion !== 'number' || envelope.schemaVersion > schemaVersion) return null
            return envelope.content ?? null
        } catch {
            return null
        }
    },

    /** Writes beside the file, then renames over it, so a crash never leaves half a file. */
    save(content, file, schemaVersion) {
        try {
            const temp = `${file}.${process.pid}.tmp`
            fs.writeFileSync(temp, JSON.stringify({ schemaVersion, content }, null, 2))
            fs.renameSync(temp, file)
        } catch (error) {
            console.error(`Couldn't save ${file}:`, error)
        }
    },
}

// MARK: - State hub

const providers = new Map()
const dirty = new Set()
let flushTimer = null

/** Sends only the parts of the state that changed, at most about fifteen times a second. */
export const Hub = {
    provide(name, read) {
        providers.set(name, read)
    },

    changed(...names) {
        for (const name of names) dirty.add(name)
        if (!flushTimer) flushTimer = setTimeout(() => Hub.flush(), 66)
    },

    flush() {
        clearTimeout(flushTimer)
        flushTimer = null
        if (!dirty.size) return
        const patch = {}
        for (const name of dirty) patch[name] = providers.get(name)?.()
        dirty.clear()
        for (const window of BrowserWindow.getAllWindows()) {
            if (!window.isDestroyed()) window.webContents.send('purrloader:state', patch)
        }
        for (const listener of Hub.listeners) listener(patch)
    },

    snapshot() {
        const state = {}
        for (const [name, read] of providers) state[name] = read()
        return state
    },

    /** Main-process listeners, such as the tray. */
    listeners: new Set(),
}

// MARK: - Settings

const DEFAULTS = {
    downloadFolder: null,
    lastOptions: null,
    maxConcurrent: 3,
    notifications: true,
    clipboardAutofill: true,
    showTrayIcon: true,
    autoUpdateYtdlp: true,
    ytdlpChannel: 'stable',
    didShowWelcome: false,
    settingsTab: 'general',
    lastAppUpdateCheck: null,
    latestAppVersion: null,
    latestAppPage: null,
    dismissedAppVersion: null,
}

/** Settings that aren't per-download, kept in settings.json. */
export const Settings = {
    values: { ...DEFAULTS },
    saveTimer: null,

    load() {
        const saved = JSONFile.load(AppPaths.settings, 1)
        if (saved && typeof saved === 'object') {
            for (const key of Object.keys(DEFAULTS)) if (Object.hasOwn(saved, key)) this.values[key] = saved[key]
        }
    },

    get(key) {
        return this.values[key]
    },

    set(key, value) {
        if (!Object.hasOwn(DEFAULTS, key)) throw new Error(`Unknown setting ${key}`)
        this.values[key] = value
        clearTimeout(this.saveTimer)
        this.saveTimer = setTimeout(() => this.save(), 250)
        Hub.changed('settings')
    },

    save() {
        clearTimeout(this.saveTimer)
        JSONFile.save(this.values, AppPaths.settings, 1)
    },

    /** What the windows may see; lastOptions travels in the model instead. */
    public() {
        const { lastOptions, ...rest } = this.values
        return rest
    },
}

Hub.provide('settings', () => Settings.public())
