// Everything the windows can ask the main process to do. Each window gets state through the hub and sends
// actions here by name; nothing else crosses over.

import { BrowserWindow, app, clipboard, dialog, ipcMain, shell } from 'electron'
import fs from 'node:fs'

import { defaultOptions } from '../core/options.js'
import { browserNote, installedBrowsers, SUPPORTED_BROWSERS } from './browsers.js'
import { Downloads, History, commandLine } from './downloads.js'
import { AppInfo, Model, Presets, Updater } from './model.js'
import { which } from './process.js'
import { AppPaths, Hub, Settings } from './store.js'
import { Tools } from './tools.js'
import { Windows } from './windows.js'

const platform = process.platform

/** Words that differ by platform, so the interface reads right on each. */
function platformInfo() {
    return {
        platform,
        version: AppInfo.version,
        repositoryURL: AppInfo.repositoryURL,
        licenseURL: AppInfo.fileURL('LICENSE'),
        noticesURL: AppInfo.fileURL('THIRD_PARTY_NOTICES.md'),
        installedBrowsers: installedBrowsers(),
        supportedBrowsers: SUPPORTED_BROWSERS,
        browserNote: browserNote(),
        fileManager: platform === 'win32' ? 'Explorer' : platform === 'darwin' ? 'Finder' : 'Files',
        trayName: platform === 'win32' ? 'notification area' : platform === 'darwin' ? 'menu bar' : 'system tray',
        mod: platform === 'darwin' ? '⌘' : 'Ctrl+',
        archivePath: AppPaths.archive,
    }
}

Hub.provide('app', platformInfo)

const job = (id) => {
    const found = Downloads.find(id)
    if (!found) throw new Error('No such download')
    return found
}

const SETTABLE = new Set(['notifications', 'clipboardAutofill', 'showTrayIcon', 'didShowWelcome', 'settingsTab'])

const actions = {
    getState: () => Hub.snapshot(),

    // Main window
    'model.setUrlText': (_, text) => Model.setUrlText(text),
    'model.refetch': () => Model.refetch(),
    'model.download': () => Model.download(),
    'model.downloadWhenStreamStarts': () => Model.downloadWhenStreamStarts(),
    'model.pasteAndFetch': () => Model.pasteAndFetch(),
    'model.acceptDropped': (_, dropped) =>
        Model.acceptDropped({
            files: (dropped?.files ?? []).filter((file) => typeof file === 'string'),
            strings: (dropped?.strings ?? []).filter((text) => typeof text === 'string'),
        }),
    'model.windowFocused': () => Model.pasteFromClipboardIfUseful(),
    'model.setOption': (_, keyPath, value) => Model.setOption(String(keyPath), value),
    'model.resetOptions': () => Model.replaceOptions(defaultOptions()),
    'model.toggleEntry': (_, index) => Model.toggleEntry(Number(index)),
    'model.selectEntries': (_, all) => Model.selectEntries(Boolean(all)),
    'model.setSubtitlesOn': (_, on) => Model.setSubtitlesOn(Boolean(on)),
    'model.setSubtitleSelection': (_, code) => Model.setSubtitleSelection(String(code)),
    'model.setSkipSponsors': (_, on) => Model.setSkipSponsors(Boolean(on)),
    'model.setDownloadPart': (_, on) => Model.setDownloadPart(Boolean(on)),
    'model.setCookieBrowser': (_, browser, refetch = true) => Model.setCookieBrowser(String(browser), { refetch }),
    'model.chooseFolder': (event) => Model.chooseFolder(BrowserWindow.fromWebContents(event.sender)),
    'model.setExactFormat': (_, exact, mode) => {
        Model.options.format.exactFormat = String(exact)
        if (mode === 'video' || mode === 'audio') Model.options.format.mode = mode
        Model.optionsChanged({ bulk: true })
    },

    // Downloads
    'jobs.pause': (_, id) => Downloads.pause(job(id)),
    'jobs.resume': (_, id) => Downloads.resume(job(id)),
    'jobs.cancel': (_, id) => Downloads.cancel(job(id)),
    'jobs.retry': (_, id, adjust = {}) =>
        Downloads.retry(job(id), (options) => {
            if (typeof adjust.cookieBrowser === 'string') options.network.cookieBrowser = adjust.cookieBrowser
            if (adjust.bestQuality) {
                options.format.maxHeight = null
                options.format.exactFormat = ''
            }
            if (adjust.waitForVideo) options.extras.waitForVideo = true
        }),
    'jobs.chooseFolderAndRetry': async (event, id) => {
        const target = job(id)
        const folder = await Model.pickFolder(BrowserWindow.fromWebContents(event.sender), target.folder)
        if (!folder) return
        Model.folder = folder
        Downloads.retry(target, () => {}, folder)
    },
    'jobs.updateAndRetry': async (_, id) => {
        await Tools.updateYtdlp()
        Downloads.retry(job(id))
    },
    'jobs.remove': (_, id) => Downloads.remove(job(id)),
    'jobs.clearRecent': () => Downloads.clearRecent(),
    'jobs.pauseAll': () => Downloads.pauseAll(),
    'jobs.resumeAll': () => Downloads.resumeAll(),
    'jobs.reveal': (_, id) => Downloads.reveal(job(id)),
    'jobs.open': (_, id) => Downloads.open(job(id)),
    'jobs.copyLink': (_, id) => clipboard.writeText(job(id).url),
    'jobs.details': (_, id) => {
        const found = Downloads.find(id)
        if (!found) return null
        return {
            title: found.title,
            statusText: found.statusText,
            log: found.log,
            commandLine: found.command.length ? commandLineOf(found) : '',
        }
    },

    // Helper tools
    'tools.retrySetup': async () => {
        await Tools.bootstrap()
        Downloads.schedule()
    },
    'tools.update': () => Tools.updateYtdlp(),
    'tools.dismissUpdateError': () => {
        Tools.updateError = null
        Tools.changed()
    },
    'tools.reinstall': async () => {
        await Tools.reinstallAll()
        Downloads.schedule()
    },
    'tools.setChannel': (_, channel) => Tools.setChannel(channel),
    'tools.setAutoUpdate': (_, on) => Settings.set('autoUpdateYtdlp', Boolean(on)),
    'tools.showFolder': () => shell.openPath(AppPaths.bin),

    // Settings
    'settings.set': (_, key, value) => {
        if (!SETTABLE.has(key)) throw new Error(`Setting ${key} can't be changed from a window`)
        Settings.set(key, value)
    },
    'settings.setMaxConcurrent': (_, value) => Downloads.setMaxConcurrent(Number(value)),

    // Saved setups
    'presets.apply': (_, id) => Presets.apply(id),
    'presets.add': (_, name) => String(name).trim() && Presets.add(String(name).trim()),
    'presets.rename': (_, id, name) => Presets.rename(id, String(name)),
    'presets.delete': (_, id) => Presets.delete(id),
    'presets.move': (_, from, to) => Presets.move(Number(from), Number(to)),
    'presets.restore': () => Presets.restoreBuiltIns(),

    // History
    'history.delete': (_, ids) => History.delete(ids),
    'history.clear': () => History.clear(),
    'history.redownload': (_, ids) => History.redownload(ids),
    'history.reveal': (_, id) => History.reveal(id),
    'history.copyLinks': (_, ids) =>
        clipboard.writeText(History.entries.filter((entry) => ids.includes(entry.id)).map((entry) => entry.url).join('\n')),

    // Download archive
    'archive.count': () => {
        try {
            return fs.readFileSync(AppPaths.archive, 'utf8').split('\n').filter(Boolean).length
        } catch {
            return 0
        }
    },
    'archive.show': () => shell.showItemInFolder(AppPaths.archive),
    'archive.clear': () => fs.rmSync(AppPaths.archive, { force: true }),

    // Network
    'network.findAria2c': () => which('aria2c'),
    'network.chooseCookiesFile': async (event) => {
        const result = await dialog.showOpenDialog(BrowserWindow.fromWebContents(event.sender), {
            properties: ['openFile'],
            filters: [
                { name: 'Cookies', extensions: ['txt'] },
                { name: 'All Files', extensions: ['*'] },
            ],
        })
        if (result.canceled || !result.filePaths[0]) return null
        Model.setOption('network.cookiesFile', result.filePaths[0])
        return result.filePaths[0]
    },

    // Windows
    'window.openSettings': (_, tab) => Windows.showSettings(tab),
    'window.openHistory': () => Windows.showHistory(),
    'window.showMain': () => {
        Windows.showMain()
        Windows.hideTrayPanel()
    },
    'window.resizeSettings': (_, tab) => Windows.resizeSettings(tab),
    'window.fitTray': (_, height) => Windows.fitTrayPanel(Number(height)),
    'window.close': (event) => BrowserWindow.fromWebContents(event.sender)?.close(),
    'app.quit': () => app.quit(),

    // App updates
    'updater.dismiss': () => Updater.dismiss(),

    // Odds and ends
    copy: (_, text) => clipboard.writeText(String(text)),
    openExternal: (_, url) => {
        if (/^https:\/\//.test(url)) shell.openExternal(url)
    },
    /** A native confirmation, like the Mac's alerts. Returns the index of the button pressed. */
    confirm: async (event, { message, detail, buttons }) => {
        const window = BrowserWindow.fromWebContents(event.sender)
        const result = await dialog.showMessageBox(window, {
            type: 'warning',
            message,
            detail,
            buttons,
            defaultId: 0,
            cancelId: buttons.length - 1,
            noLink: true,
        })
        return result.response
    },
}

function commandLineOf(found) {
    return commandLine(found.command)
}


export function registerIPC() {
    ipcMain.handle('purrloader:invoke', async (event, action, ...args) => {
        const handler = actions[action]
        if (!handler) throw new Error(`Unknown action ${action}`)
        const result = await handler(event, ...args)
        // Answer with the freshest state, so a window never waits for the next tick to see its own change.
        Hub.flush()
        return result
    })
}
