// Purrloader for Windows and Linux: app start-up, quitting, and the pieces that tie the windows to the queue.
// The Mac app is the native SwiftUI one in Sources/; this mirrors it.

import { Menu, app, dialog, nativeTheme } from 'electron'
import path from 'node:path'

import { Downloads, Hardware, History } from './downloads.js'
import { registerIPC } from './ipc.js'
import { Model, Presets, Updater } from './model.js'
import { AppPaths, Hub, Settings } from './store.js'
import { Tools } from './tools.js'
import { TrayIcon, Windows, handleProtocol, registerScheme } from './windows.js'

// On a Mac this build is only for development; keep it away from the real Mac app's files.
if (process.platform === 'darwin') {
    app.setPath('userData', path.join(app.getPath('appData'), 'Purrloader (Electron)'))
}
// Chromium's caches go in a subfolder, so the app data folder holds only Purrloader's own files.
app.setPath('sessionData', path.join(app.getPath('userData'), 'Chromium'))
app.setAppUserModelId('com.blcnyy.purrloader')
registerScheme()

if (!app.requestSingleInstanceLock()) {
    app.quit()
} else {
    app.on('second-instance', () => Windows.showMain())
    app.whenReady()
        .then(start)
        .catch((error) => {
            console.error('Purrloader failed to start:', error)
            dialog.showErrorBox('Purrloader couldn’t start', String(error?.stack ?? error))
            app.exit(1)
        })
}

async function start() {
    nativeTheme.themeSource = 'dark'
    AppPaths.init()
    Settings.load()
    Model.load()
    Presets.load()
    History.load()
    Downloads.restoreQueue()
    handleProtocol()
    registerIPC()
    setMenu()

    Windows.onMainFocus = () => Model.pasteFromClipboardIfUseful()
    Downloads.onActivate = () => Windows.showMain()
    Windows.showMain()
    TrayIcon.init()
    detectAV1()

    Tools.bootstrap().then(() => Downloads.schedule())
    Updater.refresh()
    if (!app.isPackaged && process.env.PURRLOADER_SNAPSHOT) {
        import('./dev-snapshot.js')
            .then((dev) => dev.runSnapshots(process.env.PURRLOADER_SNAPSHOT, process.env.PURRLOADER_SCENARIO))
            .catch((error) => console.error('Snapshot run failed:', error))
    }
    // Long sessions still hear about new versions; checkIfDue asks GitHub at most once a day.
    const check = () => Updater.checkIfDue()
    check()
    setInterval(check, 6 * 3600 * 1000)
}

/** Windows and Linux have no menu bar in a Purrloader window (the title bar menu button has it all); the Mac
 * development build gets the standard menus so editing shortcuts work. */
function setMenu() {
    if (process.platform !== 'darwin') return Menu.setApplicationMenu(null)
    Menu.setApplicationMenu(
        Menu.buildFromTemplate([{ role: 'appMenu' }, { role: 'editMenu' }, { role: 'windowMenu' }]),
    )
}

/** Players play AV1 smoothly only where the graphics chip decodes it. Chromium knows which codecs those are. */
async function detectAV1() {
    try {
        const info = await app.getGPUInfo('complete')
        const profiles = JSON.stringify(info?.auxAttributes?.videoDecodeAcceleratorSupportedProfile ?? info ?? '')
        Hardware.playsAV1 = /av1/i.test(profiles)
    } catch {
        Hardware.playsAV1 = true
    }
    Hub.changed('model')
}

// MARK: - Quitting

let confirmedQuit = false

app.on('before-quit', (event) => {
    if (confirmedQuit) return
    const pending = Downloads.activeCount + Downloads.queuedCount
    if (pending > 0) {
        event.preventDefault()
        const parent = Windows.main?.isVisible() ? Windows.main : undefined
        dialog
            .showMessageBox(parent, {
                type: 'warning',
                message: pending === 1 ? 'A download is still in progress' : `${pending} downloads are still in progress`,
                detail: `If you quit, Purrloader pauses ${pending === 1 ? 'it' : 'them'} and picks up where it left off next time you open it.`,
                buttons: ['Quit', 'Keep Downloading'],
                defaultId: 0,
                cancelId: 1,
                noLink: true,
            })
            .then(({ response }) => {
                if (response === 0) quitNow()
            })
        return
    }
    confirmedQuit = true
    app.isQuitting = true
    Downloads.stopAllForQuit()
    Settings.save()
})

/** Pauses running downloads, gives yt-dlp a moment to close its files, then quits. */
function quitNow() {
    confirmedQuit = true
    app.isQuitting = true
    const running = Downloads.jobs.map((job) => job.process).filter(Boolean)
    Downloads.stopAllForQuit()
    Settings.save()
    Promise.race([Promise.all(running.map((child) => child.exited)), new Promise((resolve) => setTimeout(resolve, 3000))]).then(
        () => app.quit(),
    )
}

app.on('window-all-closed', () => {
    // With the tray icon on, Purrloader keeps running; otherwise closing the last window quits.
    if (!TrayIcon.current) app.quit()
})

app.on('activate', () => Windows.showMain())
