// The download queue and history. A port of the Mac app's DownloadManager.swift and History.swift.

import { Notification, shell } from 'electron'
import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { YtDlpCommand } from '../core/command.js'
import { friendlyError, friendlyErrorFromRaw } from '../core/errors.js'
import { cloneOptions, decodeOptions } from '../core/options.js'
import { DownloadProgress } from '../core/progress.js'
import { Format, Shell } from '../core/support.js'
import { ChildProcess, which } from './process.js'
import { AppPaths, Hub, JSONFile, Settings } from './store.js'
import { Tools } from './tools.js'

const ACTIVE = new Set(['starting', 'downloading', 'processing'])
const DONE = new Set(['finished', 'cancelled', 'failed'])
export const isActive = (status) => ACTIVE.has(status)
/** Finished and cancelled jobs sit under "Recent"; failed ones stay up top until dealt with. */
const isRecent = (status) => status === 'finished' || status === 'cancelled'

/** Whether this computer decodes AV1 in hardware; set once the GPU has been asked (see main.js). */
export const Hardware = { playsAV1: true }

/** The tools' paths for a command. Looks for aria2c on disk only when the options want it. */
export async function commandEnvironment(options) {
    return {
        ffmpeg: Tools.path('ffmpeg') ?? 'ffmpeg',
        deno: Tools.path('deno') ?? 'deno',
        archive: AppPaths.archive,
        aria2c: options.network.useAria2c ? await which('aria2c') : null,
        playsAV1: Hardware.playsAV1,
        platform: process.platform,
        home: os.homedir(),
    }
}

/** The same, without looking on disk, for previews that must answer straight away. */
export function commandEnvironmentNow() {
    return {
        ffmpeg: Tools.path('ffmpeg') ?? 'ffmpeg',
        deno: Tools.path('deno') ?? 'deno',
        archive: AppPaths.archive,
        aria2c: 'aria2c',
        playsAV1: Hardware.playsAV1,
        platform: process.platform,
        home: os.homedir(),
    }
}

export function commandLine(args) {
    return Shell.commandLine(args, process.platform)
}

class DownloadJob {
    constructor({ url, title, thumbnail, options, folder, playlistItems = null }) {
        this.id = randomUUID()
        this.url = url
        this.title = title || url
        this.thumbnail = thumbnail ?? null
        this.options = options
        this.folder = folder
        /** Ticked playlist items in yt-dlp -I syntax; null downloads what the options say. */
        this.playlistItems = playlistItems
        this.status = 'queued'
        this.error = null
        this.progress = new DownloadProgress()
        this.log = []
        this.files = []
        this.process = null
        this.stopReason = null
        /** A blocked download gets one automatic yt-dlp update and retry. */
        this.didAutoUpdate = false
        this.command = [Tools.path('yt-dlp') ?? 'yt-dlp', ...this.makeCommand(commandEnvironmentNow()).download(url, folder, { playlistItems })]
    }

    makeCommand(environment) {
        return new YtDlpCommand(this.options, environment)
    }

    get fraction() {
        return this.status === 'finished' ? 1 : this.progress.fraction
    }

    get currentItemTitle() {
        return this.progress.isPlaylist ? this.progress.reportedTitle : null
    }

    appendLog(line) {
        this.log.push(line)
        if (this.log.length > 4000) this.log.splice(0, this.log.length - 4000)
    }

    get statusText() {
        const p = this.progress
        switch (this.status) {
            case 'queued':
                return 'Waiting…'
            case 'paused':
                return p.totalBytes > 0 ? `Paused · ${Format.bytes(p.downloadedBytes)} of ${Format.bytes(p.totalBytes)}` : 'Paused'
            case 'cancelled':
                return 'Cancelled'
            case 'failed':
                return this.error?.message ?? 'Something went wrong.'
            case 'finished':
                if (this.files.length > 1) return `Done · ${this.files.length} files`
                return p.totalBytes > 0 ? `Done · ${Format.bytes(p.totalBytes)}` : 'Done'
            case 'starting':
            case 'processing':
                return p.stage || 'Working…'
            case 'downloading': {
                const parts = []
                if (p.itemIndex != null && p.itemCount != null) parts.push(`Item ${p.itemIndex} of ${p.itemCount}`)
                if (p.totalBytes > 0) parts.push(`${Format.bytes(p.downloadedBytes)} of ${Format.bytes(p.totalBytes)}`)
                else if (p.downloadedBytes > 0) parts.push(Format.bytes(p.downloadedBytes))
                if (p.speed > 0) parts.push(`${Format.bytes(p.speed)}/s`)
                const left = p.eta > 0 ? Format.duration(p.eta) : null
                if (left) parts.push(`${left} left`)
                if (p.stage) parts.push(p.stage)
                return parts.length ? parts.join(' · ') : 'Downloading…'
            }
        }
        return ''
    }

    summary() {
        return {
            id: this.id,
            url: this.url,
            title: this.title,
            thumbnail: this.thumbnail,
            status: this.status,
            error: this.error,
            statusText: this.statusText,
            fraction: this.fraction,
            currentItemTitle: this.currentItemTitle,
            isActive: isActive(this.status),
            isRecent: isRecent(this.status),
            isDone: DONE.has(this.status),
            hasFiles: this.files.length > 0,
            folder: this.folder,
            cookieBrowser: this.options.network.cookieBrowser,
            logLength: this.log.length,
        }
    }
}

export const Downloads = {
    queueSchemaVersion: 1,
    /** Newest first. */
    jobs: [],
    askedForNotifications: false,

    get maxConcurrent() {
        return Math.max(1, Number(Settings.get('maxConcurrent')) || 1)
    },

    setMaxConcurrent(value) {
        Settings.set('maxConcurrent', Math.min(10, Math.max(1, Math.round(value))))
        this.schedule()
    },

    get activeCount() {
        return this.jobs.filter((job) => isActive(job.status)).length
    },

    get queuedCount() {
        return this.jobs.filter((job) => job.status === 'queued').length
    },

    /** Average progress of active jobs, for the tray and the taskbar. */
    get overallFraction() {
        const active = this.jobs.filter((job) => isActive(job.status))
        if (!active.length) return null
        return active.reduce((sum, job) => sum + (job.fraction ?? 0), 0) / active.length
    },

    find(id) {
        return this.jobs.find((job) => job.id === id)
    },

    create(fields) {
        return new DownloadJob(fields)
    },

    enqueue(job) {
        this.jobs.unshift(job)
        this.schedule()
    },

    schedule() {
        if (!Tools.isReady) return this.stateChanged()
        // Oldest queued first; jobs are stored newest first.
        while (this.activeCount < this.maxConcurrent) {
            const next = this.jobs.findLast((job) => job.status === 'queued')
            if (!next) break
            this.start(next)
        }
        this.stateChanged()
    },

    async start(job) {
        job.status = 'starting'
        job.error = null
        job.progress = new DownloadProgress()
        job.stopReason = null
        job.files = []

        const filepathLog = path.join(AppPaths.temp, `${job.id}.paths`)
        fs.rmSync(filepathLog, { force: true })
        const environment = await commandEnvironment(job.options)
        const args = job.makeCommand(environment).download(job.url, job.folder, {
            playlistItems: job.playlistItems,
            filepathLog,
        })
        const executable = Tools.path('yt-dlp')
        job.command = [executable, ...args]
        job.appendLog('$ ' + commandLine(job.command))
        // Paused or cancelled while the environment was being looked up.
        if (job.status !== 'starting') return

        const child = new ChildProcess(executable, args, {
            // Not the download folder, so yt-dlp's working directory never blocks removing a USB drive.
            cwd: AppPaths.temp,
            onLine: (line) => this.handle(line, job),
        })
        job.process = child
        const code = await child.exited
        if (child.error) job.appendLog(`Couldn't start yt-dlp: ${child.error.message}`)
        this.finish(job, code, filepathLog)
    },

    handle(line, job) {
        if (job.progress.consume(line)) job.appendLog(line)
        if (!isActive(job.status)) return
        job.status = job.progress.phase
        if (!job.progress.isPlaylist && job.title === job.url && job.progress.reportedTitle) {
            job.title = job.progress.reportedTitle
        }
        Hub.changed('jobs')
    },

    finish(job, code, filepathLog) {
        job.process = null
        try {
            job.files = fs.readFileSync(filepathLog, 'utf8').split(/\r?\n/).filter(Boolean)
            fs.rmSync(filepathLog, { force: true })
        } catch {}

        if (job.stopReason) {
            job.status = job.stopReason
            if (job.stopReason === 'cancelled') removePartials(job)
        } else if (code === 0) {
            job.status = 'finished'
            const sizes = job.files.map((file) => safeSize(file)).filter((size) => size != null)
            if (sizes.length) job.progress.totalBytes = sizes.reduce((a, b) => a + b, 0)
            History.record(job)
            this.notify('Downloaded', job.title, job.files[0])
        } else {
            const error = job.progress.lastError ? friendlyErrorFromRaw(job.progress.lastError) : friendlyError('unexpectedExit')
            if (error.kind === 'blocked' && !job.didAutoUpdate) return this.updateAndRetry(job)
            job.status = 'failed'
            job.error = error
            this.notify("Couldn't download", `${job.title}\n${error.message}`)
        }
        this.schedule()
    },

    /** Sites often block old yt-dlp versions; a fresh one usually gets through. */
    async updateAndRetry(job) {
        job.didAutoUpdate = true
        job.status = 'processing'
        job.progress.stage = 'Updating helper tools, then trying again…'
        job.appendLog('[purrloader] Blocked; updating yt-dlp and retrying once.')
        this.schedule()
        await Tools.updateYtdlp({ silently: true })
        // Unless it was cancelled or paused meanwhile.
        if (job.status !== 'processing' || job.process) return
        job.status = 'queued'
        this.schedule()
    },

    // MARK: Controls

    /** Pausing stops yt-dlp; resuming restarts it and yt-dlp continues the .part files. */
    pause(job) {
        if (job.status === 'queued') {
            job.status = 'paused'
            return this.stateChanged()
        }
        this.stop(job, 'paused')
        this.stateChanged()
    },

    resume(job) {
        if (job.status !== 'paused') return
        job.status = 'queued'
        this.schedule()
    },

    cancel(job) {
        if (isActive(job.status)) {
            this.stop(job, 'cancelled')
        } else {
            job.status = 'cancelled'
            removePartials(job)
        }
        this.stateChanged()
    },

    /** Retries, optionally with changed settings: a browser to sign in with, a different folder. */
    retry(job, adjust = () => {}, folder = null) {
        if (isActive(job.status)) return
        adjust(job.options)
        if (folder) job.folder = folder
        job.status = 'queued'
        job.error = null
        job.progress = new DownloadProgress()
        this.schedule()
    },

    remove(job) {
        if (isActive(job.status)) return
        this.jobs = this.jobs.filter((other) => other.id !== job.id)
        this.stateChanged()
    },

    clearRecent() {
        this.jobs = this.jobs.filter((job) => !isRecent(job.status))
        this.stateChanged()
    },

    pauseAll() {
        for (const job of this.jobs.filter((job) => isActive(job.status) || job.status === 'queued')) this.pause(job)
    },

    resumeAll() {
        for (const job of this.jobs.filter((job) => job.status === 'paused')) this.resume(job)
    },

    /** Stops running downloads (yt-dlp keeps the .part files) and saves them as queued, so they pick up where
     * they left off next launch. */
    stopAllForQuit() {
        this.saveQueue()
        for (const job of this.jobs.filter((job) => isActive(job.status))) {
            job.stopReason = 'paused'
            job.process?.interrupt()
        }
    },

    reveal(job) {
        revealFiles(job.files, job.folder)
    },

    open(job) {
        const file = job.files.find((file) => fs.existsSync(file))
        if (file) shell.openPath(file)
    },

    stop(job, reason) {
        const child = job.process
        if (!child?.running) {
            job.status = reason
            if (reason === 'cancelled') removePartials(job)
            return
        }
        job.stopReason = reason
        job.progress.stage = reason === 'paused' ? 'Pausing…' : 'Cancelling…'
        child.interrupt()
        setTimeout(() => child.running && child.terminate(), 5000).unref()
    },

    // MARK: Persistence

    saveQueue() {
        const saved = this.jobs.slice(0, 200).map((job) => ({
            url: job.url,
            title: job.title,
            thumbnail: job.thumbnail,
            options: job.options,
            folder: job.folder,
            playlistItems: job.playlistItems,
            // A running job is saved as queued: if Purrloader quits now it resumes next launch.
            status: isActive(job.status) ? 'queued' : job.status,
            error: job.error,
            files: job.files,
            totalBytes: job.progress.totalBytes,
        }))
        JSONFile.save(saved, AppPaths.queue, this.queueSchemaVersion)
    },

    restoreQueue() {
        const saved = JSONFile.load(AppPaths.queue, this.queueSchemaVersion)
        if (!Array.isArray(saved)) return
        this.jobs = saved
            .filter((item) => typeof item?.url === 'string')
            .map((item) => {
                const job = new DownloadJob({
                    url: item.url,
                    title: item.title,
                    thumbnail: item.thumbnail,
                    options: decodeOptions(item.options),
                    folder: item.folder || AppPaths.defaultDownloads,
                    playlistItems: item.playlistItems ?? null,
                })
                job.files = Array.isArray(item.files) ? item.files : []
                job.progress.totalBytes = item.totalBytes || 0
                // Interrupted by quit: resumes once tools are ready.
                job.status = ['queued', 'paused', 'finished', 'cancelled', 'failed'].includes(item.status) ? item.status : 'queued'
                if (job.status === 'failed') job.error = item.error ?? friendlyError('unexpectedExit')
                return job
            })
    },

    /** Saves the queue and updates the badge and taskbar progress. */
    stateChanged() {
        this.saveQueue()
        Hub.changed('jobs')
        this.onChange?.()
    },

    // MARK: Notifications

    /** Clicking a "Downloaded" notification shows the file in its folder. */
    notify(title, body, file = null) {
        if (!Settings.get('notifications') || !Notification.isSupported()) return
        const notification = new Notification({ title, body, silent: false })
        notification.on('click', () => {
            if (file && fs.existsSync(file)) shell.showItemInFolder(file)
            else this.onActivate?.()
        })
        notification.show()
    },

    state() {
        return {
            jobs: this.jobs.map((job) => job.summary()),
            activeCount: this.activeCount,
            queuedCount: this.queuedCount,
            overallFraction: this.overallFraction,
            hasPaused: this.jobs.some((job) => job.status === 'paused'),
        }
    },
}

function safeSize(file) {
    try {
        return fs.statSync(file).size
    } catch {
        return null
    }
}

export function revealFiles(files, folder) {
    const existing = files.find((file) => fs.existsSync(file))
    if (existing) shell.showItemInFolder(existing)
    else shell.openPath(folder)
}

function removePartials(job) {
    for (const destination of job.progress.destinations) {
        const file = path.isAbsolute(destination) ? destination : path.join(job.folder, destination)
        for (const suffix of ['.part', '.ytdl', '.part.ytdl']) fs.rmSync(file + suffix, { force: true })
        const dir = path.dirname(file)
        const name = path.basename(file)
        for (const entry of safeReaddir(dir)) {
            if (entry.startsWith(name + '.part-Frag')) fs.rmSync(path.join(dir, entry), { force: true })
        }
    }
}

function safeReaddir(dir) {
    try {
        return fs.readdirSync(dir)
    } catch {
        return []
    }
}

Hub.provide('jobs', () => Downloads.state())

// MARK: - History

export const History = {
    schemaVersion: 1,
    entries: [],

    load() {
        const saved = JSONFile.load(AppPaths.history, this.schemaVersion)
        this.entries = Array.isArray(saved) ? saved.filter((entry) => typeof entry?.url === 'string') : []
    },

    save() {
        JSONFile.save(this.entries, AppPaths.history, this.schemaVersion)
        Hub.changed('history')
    },

    record(job) {
        this.entries.unshift({
            id: randomUUID(),
            url: job.url,
            title: job.title,
            date: new Date().toISOString(),
            folder: job.folder,
            files: job.files,
            thumbnail: job.thumbnail,
            options: job.options,
        })
        this.save()
    },

    delete(ids) {
        const remove = new Set(ids)
        this.entries = this.entries.filter((entry) => !remove.has(entry.id))
        this.save()
    },

    clear() {
        this.entries = []
        this.save()
    },

    redownload(ids) {
        for (const entry of this.entries.filter((entry) => ids.includes(entry.id))) {
            Downloads.enqueue(
                Downloads.create({
                    url: entry.url,
                    title: entry.title,
                    thumbnail: entry.thumbnail,
                    options: cloneOptions(decodeOptions(entry.options)),
                    folder: entry.folder,
                }),
            )
        }
    },

    reveal(id) {
        const entry = this.entries.find((entry) => entry.id === id)
        if (entry) revealFiles(entry.files ?? [], entry.folder)
    },

    state() {
        return this.entries.map(({ options, ...entry }) => entry)
    },
}

Hub.provide('history', () => History.state())
