// State of the main window (the link being looked at and the options for the next download), saved setups, and
// the app update check. Ports of the Mac app's MainModel.swift, PresetStore.swift and AppUpdater.swift.

import { app, clipboard, dialog, net } from 'electron'
import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

import { YtDlpCommand } from '../core/command.js'
import { friendlyError, friendlyErrorFromRaw } from '../core/errors.js'
import { defaultSubtitleCode, parseMediaInfo, renderTemplate, subtitleTrack } from '../core/media.js'
import {
    DEFAULT_SPONSOR_CATEGORIES,
    cloneOptions,
    decodeOptions,
    defaultOptions,
    expectedExtension,
    setPath,
} from '../core/options.js'
import { compressPlaylistSelection, displayVersion, isNewerVersion, webLinks } from '../core/support.js'
import { Downloads, commandEnvironment, commandEnvironmentNow, commandLine } from './downloads.js'
import { ChildProcess } from './process.js'
import { AppPaths, Hub, JSONFile, Settings } from './store.js'
import { Tools } from './tools.js'

const locale = () => app.getLocale() || 'en'
const language = () => locale().split(/[-_]/)[0].toLowerCase()
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

export const Model = {
    urlText: '',
    /** Bumped whenever the main process changes the link itself, so the field takes the new text. */
    urlRevision: 0,
    info: null,
    infoRaw: null,
    isFetching: false,
    fetchError: null,
    selectedEntries: new Set(),
    /** The subtitle track picked for this link; null uses the saved language. */
    subtitleLanguage: null,
    downloadPart: false,
    options: defaultOptions(),
    /** Bumped when options change all at once (reset, a saved setup), so text fields take the new values. */
    optionsRevision: 0,
    fetchToken: 0,
    fetchTimer: null,
    fetchChild: null,
    lastClipboardLink: null,

    load() {
        this.options = decodeOptions(Settings.get('lastOptions'))
    },

    get folder() {
        const saved = Settings.get('downloadFolder')
        return typeof saved === 'string' && saved ? saved : AppPaths.defaultDownloads
    },

    set folder(value) {
        Settings.set('downloadFolder', value)
        this.changed()
    },

    get links() {
        return webLinks(this.urlText)
    },

    get canDownload() {
        return (
            this.links.length > 0 &&
            Tools.isReady &&
            !this.fetchError &&
            !(this.info?.isPlaylist && this.selectedEntries.size === 0)
        )
    },

    /** Quality tiers for the picker. With formats known, only tiers the video has: a tier shows when some stream
     * is taller than the tier below it, so a 1920×800 film offers 1080p. */
    get qualityHeights() {
        const tiers = [2160, 1440, 1080, 720, 480]
        const heights = this.info?.availableHeights ?? []
        if (!heights.length) return tiers
        return tiers.filter((tier, index) => {
            const below = tiers[index + 1] ?? 0
            return heights.some((height) => height > below && height <= tier)
        })
    },

    /** Languages offered in the More popover: this video's tracks, or the system language and English. */
    get subtitleChoices() {
        const tracks = this.info?.subtitleTracks
        if (tracks?.length) return tracks
        return [...new Set([language(), 'en'])].map((code) => subtitleTrack(`${code}.*`, false, locale()))
    },

    get subtitleSelection() {
        return this.subtitleLanguage ?? this.subtitleChoices[0]?.code ?? 'en.*'
    },

    get skipSponsors() {
        return this.options.extras.sponsorBlock === 'remove'
    },

    /** How many More popover options are on, shown on its chip. */
    get activeMoreOptions() {
        return [
            this.options.subtitles.enabled,
            this.skipSponsors,
            this.downloadPart,
            this.options.network.cookieBrowser !== 'none',
        ].filter(Boolean).length
    },

    changed() {
        Hub.changed('model')
    },

    optionsChanged({ bulk = false } = {}) {
        if (bulk) this.optionsRevision++
        Settings.set('lastOptions', this.options)
        this.changed()
    },

    setOption(keyPath, value) {
        setPath(this.options, keyPath, value)
        this.options = decodeOptions(this.options)
        this.optionsChanged()
    },

    replaceOptions(options) {
        this.options = decodeOptions(options)
        this.optionsChanged({ bulk: true })
    },

    setSubtitlesOn(on) {
        this.setOption('subtitles.enabled', on)
    },

    setSubtitleSelection(code) {
        this.subtitleLanguage = code
        this.setOption('subtitles.languages', code)
    },

    setSkipSponsors(on) {
        this.options.extras.sponsorBlock = on ? 'remove' : 'off'
        if (on && !this.options.extras.sponsorCategories.length) {
            this.options.extras.sponsorCategories = [...DEFAULT_SPONSOR_CATEGORIES]
        }
        this.optionsChanged()
    },

    setDownloadPart(on) {
        this.downloadPart = on
        this.changed()
    },

    setCookieBrowser(browser, { refetch = true } = {}) {
        this.setOption('network.cookieBrowser', browser)
        if (refetch && (this.info || this.fetchError)) this.refetch()
    },

    // MARK: Fetching

    setUrlText(text, { fromMain = false } = {}) {
        this.urlText = String(text ?? '')
        if (fromMain) this.urlRevision++
        this.linkChanged()
    },

    linkChanged() {
        this.cancelFetch()
        this.info = null
        this.infoRaw = null
        this.fetchError = null
        if (this.options.format.exactFormat) {
            this.options.format.exactFormat = ''
            this.optionsChanged({ bulk: true })
        }
        this.subtitleLanguage = null
        this.isFetching = false
        this.changed()
        const links = this.links
        // Only fetch details for a single link; several links are treated as a batch.
        if (links.length !== 1) return
        const token = this.fetchToken
        this.fetchTimer = setTimeout(() => token === this.fetchToken && this.fetch(links[0], token), 350)
    },

    cancelFetch() {
        this.fetchToken++
        clearTimeout(this.fetchTimer)
        this.fetchChild?.kill()
        this.fetchChild = null
    },

    async fetch(link, token) {
        // A link can arrive (clipboard, drop) while tools are still installing on launch.
        while (!Tools.isReady) {
            await delay(300)
            if (token !== this.fetchToken) return
        }
        this.isFetching = true
        this.fetchError = null
        this.changed()
        try {
            const environment = await commandEnvironment(this.options)
            const args = new YtDlpCommand(this.options, environment).info(link)
            const result = await ChildProcess.run(Tools.path('yt-dlp'), args, {
                cwd: AppPaths.temp,
                onSpawn: (child) => (this.fetchChild = child),
            })
            if (token !== this.fetchToken || this.links[0] !== link) return
            this.fetchChild = null
            let json = null
            if (result.status === 0) {
                try {
                    json = JSON.parse(result.stdout.toString('utf8'))
                } catch {}
            }
            if (!json || typeof json !== 'object') {
                const line = result.stderr
                    .toString('utf8')
                    .split(/\r?\n/)
                    .findLast((l) => l.startsWith('ERROR:'))
                const error = line ? friendlyErrorFromRaw(line.slice(6).trim()) : friendlyError('unsupportedLink')
                // A link picked up from the clipboard that isn't a video (a profile, an article) wasn't asked for,
                // so it goes away quietly instead of showing an error.
                if (
                    link === this.lastClipboardLink &&
                    this.urlText.trim() === link &&
                    ['unsupportedLink', 'invalidLink'].includes(error.kind)
                ) {
                    this.isFetching = false
                    return this.setUrlText('', { fromMain: true })
                }
                this.fetchError = error
                return
            }
            const info = parseMediaInfo(json, link, locale())
            this.info = info
            this.infoRaw = json
            this.selectedEntries = new Set(info.entries.map((entry) => entry.index))
            this.subtitleLanguage = defaultSubtitleCode(info, this.options.subtitles.languages, locale())
        } catch (error) {
            console.error('Fetching info failed:', error)
            if (token === this.fetchToken) this.fetchError = friendlyError('unexpectedExit')
        } finally {
            if (token === this.fetchToken) {
                this.isFetching = false
                this.changed()
            }
        }
    },

    refetch() {
        const link = this.links[0]
        if (!link) return
        this.cancelFetch()
        this.fetch(link, this.fetchToken)
    },

    // MARK: Playlist

    toggleEntry(index) {
        if (this.selectedEntries.has(index)) this.selectedEntries.delete(index)
        else this.selectedEntries.add(index)
        this.changed()
    },

    selectEntries(all) {
        this.selectedEntries = new Set(all ? (this.info?.entries ?? []).map((entry) => entry.index) : [])
        this.changed()
    },

    // MARK: Clipboard & drops

    pasteFromClipboardIfUseful() {
        if (!Settings.get('clipboardAutofill')) return
        const link = webLinks(clipboard.readText().trim())[0]
        if (!link || link === this.lastClipboardLink) return
        this.lastClipboardLink = link
        if (this.urlText.trim()) return
        this.setUrlText(link, { fromMain: true })
    },

    pasteAndFetch() {
        const text = clipboard.readText()
        if (!text) return
        this.setUrlText(text.trim(), { fromMain: true })
    },

    /** Accepts dropped web links and .txt batch files (one URL per line). */
    acceptDropped({ files = [], strings = [] }) {
        const found = []
        for (const file of files) {
            try {
                const text = fs.readFileSync(file, 'utf8')
                for (const line of text.split(/\r?\n/)) {
                    const trimmed = line.trim()
                    if (!trimmed.startsWith('#') && !trimmed.startsWith(';')) found.push(...webLinks(trimmed))
                }
            } catch {}
        }
        for (const text of strings) found.push(...webLinks(text))
        if (!found.length) return
        this.setUrlText(found.join('\n'), { fromMain: true })
    },

    // MARK: Download

    /** Options for the next download: the saved ones plus this link's picks from the More popover. */
    optionsForDownload() {
        const options = cloneOptions(this.options)
        if (options.subtitles.enabled) options.subtitles.languages = this.subtitleSelection
        if (!this.downloadPart) {
            options.extras.sectionStart = ''
            options.extras.sectionEnd = ''
        }
        return options
    },

    playlistItems() {
        const info = this.info
        if (info?.isPlaylist && this.selectedEntries.size < info.entries.length) {
            return compressPlaylistSelection([...this.selectedEntries])
        }
        return null
    },

    download() {
        if (!this.canDownload) return
        const options = this.optionsForDownload()
        const info = this.info
        if (info && this.links.length === 1) {
            if (info.isPlaylist) options.playlist.noPlaylist = false
            Downloads.enqueue(
                Downloads.create({
                    url: info.url,
                    title: info.title,
                    thumbnail: info.thumbnail,
                    options,
                    folder: this.folder,
                    playlistItems: this.playlistItems(),
                }),
            )
        } else {
            for (const link of this.links) {
                Downloads.enqueue(Downloads.create({ url: link, options: cloneOptions(options), folder: this.folder }))
            }
        }
        this.options.format.exactFormat = ''
        this.options.extras.sectionStart = ''
        this.options.extras.sectionEnd = ''
        this.downloadPart = false
        this.optionsChanged({ bulk: true })
        this.setUrlText('', { fromMain: true })
    },

    /** For a scheduled stream: queue it now and let yt-dlp wait until it goes live. */
    downloadWhenStreamStarts() {
        const link = this.links[0]
        if (!link) return
        const options = this.optionsForDownload()
        options.extras.waitForVideo = true
        Downloads.enqueue(Downloads.create({ url: link, options, folder: this.folder }))
        this.setUrlText('', { fromMain: true })
    },

    /** Asks for a folder; null if the person cancels. */
    async pickFolder(window, startingAt) {
        const result = await dialog.showOpenDialog(window, {
            title: 'Choose where Purrloader saves downloads.',
            defaultPath: startingAt,
            buttonLabel: 'Choose',
            properties: ['openDirectory', 'createDirectory', 'promptToCreate'],
        })
        return result.canceled ? null : result.filePaths[0]
    },

    async chooseFolder(window) {
        const folder = await this.pickFolder(window, this.folder)
        if (folder) this.folder = folder
        return folder
    },

    /** The command that the Download button would run right now. */
    get previewCommand() {
        const link = this.links[0] ?? 'URL'
        const args = new YtDlpCommand(this.optionsForDownload(), commandEnvironmentNow()).download(link, this.folder, {
            playlistItems: this.playlistItems(),
        })
        return commandLine([Tools.path('yt-dlp') ?? 'yt-dlp', ...args])
    },

    get templatePreview() {
        return renderTemplate(this.options.output.template, this.infoRaw, {
            ext: expectedExtension(this.options.format),
            restrict: this.options.output.restrictFilenames,
            platform: process.platform,
            locale: locale(),
        })
    },

    state() {
        return {
            urlText: this.urlText,
            urlRevision: this.urlRevision,
            linkCount: this.links.length,
            info: this.info,
            isFetching: this.isFetching,
            fetchError: this.fetchError,
            selectedEntries: [...this.selectedEntries],
            subtitleSelection: this.subtitleSelection,
            subtitleChoices: this.subtitleChoices,
            downloadPart: this.downloadPart,
            skipSponsors: this.skipSponsors,
            activeMoreOptions: this.activeMoreOptions,
            options: this.options,
            optionsRevision: this.optionsRevision,
            folder: this.folder,
            folderName: path.basename(this.folder) || this.folder,
            canDownload: this.canDownload,
            qualityHeights: this.qualityHeights,
            previewCommand: this.previewCommand,
            templatePreview: this.templatePreview,
        }
    },
}

Hub.provide('model', () => Model.state())

// MARK: - Saved setups

function builtInPresets() {
    const make = (name, configure) => {
        const options = defaultOptions()
        configure(options)
        return { id: randomUUID(), name, options }
    }
    return [
        make('Default', () => {}),
        make('1080p MP4 (H.264)', (o) => {
            o.format.maxHeight = 1080
            o.format.codec = 'h264'
        }),
        make('Podcast MP3', (o) => {
            o.format.mode = 'audio'
            o.format.audioQuality = '128K'
            o.extras.sponsorBlock = 'remove'
            o.output.template = '%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s'
        }),
        make('Archive 4K MKV', (o) => {
            o.format.maxHeight = 2160
            o.format.container = 'mkv'
            o.subtitles.enabled = true
            o.subtitles.languages = 'all,-live_chat'
            o.extras.writeInfoJSON = true
            o.extras.writeDescription = true
            o.playlist.useArchive = true
            o.output.template = '%(uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s'
        }),
        make('Music FLAC', (o) => {
            o.format.mode = 'audio'
            o.format.audioFormat = 'flac'
        }),
    ]
}

export const Presets = {
    schemaVersion: 1,
    presets: [],

    load() {
        const saved = JSONFile.load(AppPaths.presets, this.schemaVersion)
        this.presets = Array.isArray(saved)
            ? saved
                  .filter((preset) => typeof preset?.name === 'string')
                  .map((preset) => ({ id: preset.id || randomUUID(), name: preset.name, options: decodeOptions(preset.options) }))
            : builtInPresets()
    },

    save() {
        JSONFile.save(this.presets, AppPaths.presets, this.schemaVersion)
        Hub.changed('presets')
    },

    apply(id) {
        const preset = this.presets.find((preset) => preset.id === id)
        if (!preset) return
        const exact = Model.options.format.exactFormat
        const options = cloneOptions(preset.options)
        options.format.exactFormat = exact
        Model.replaceOptions(options)
    },

    /** Saves under a name, replacing a setup that already has it. */
    add(name) {
        const options = cloneOptions(Model.options)
        options.format.exactFormat = '' // stream IDs only make sense for one video
        const existing = this.presets.find((preset) => preset.name === name)
        if (existing) existing.options = options
        else this.presets.push({ id: randomUUID(), name, options })
        this.save()
    },

    rename(id, name) {
        const preset = this.presets.find((preset) => preset.id === id)
        if (preset && name.trim()) preset.name = name.trim()
        this.save()
    },

    delete(id) {
        this.presets = this.presets.filter((preset) => preset.id !== id)
        this.save()
    },

    move(from, to) {
        const [preset] = this.presets.splice(from, 1)
        if (preset) this.presets.splice(Math.max(0, Math.min(to, this.presets.length)), 0, preset)
        this.save()
    },

    restoreBuiltIns() {
        for (const preset of builtInPresets()) {
            if (!this.presets.some((other) => other.name === preset.name)) this.presets.push(preset)
        }
        this.save()
    },

    state() {
        return this.presets.map(({ id, name }) => ({ id, name }))
    },
}

Hub.provide('presets', () => Presets.state())

// MARK: - App updates

export const AppInfo = {
    repository: 'BLCNYY/Purrloader',
    get repositoryURL() {
        return `https://github.com/${this.repository}`
    },
    get version() {
        return displayVersion(app.getVersion())
    },
    fileURL(file) {
        return `${this.repositoryURL}/blob/main/${file}`
    },
}

/** Looks for a newer Purrloader on GitHub Releases at most once a day. */
export const Updater = {
    available: null,

    async checkIfDue() {
        const last = Date.parse(Settings.get('lastAppUpdateCheck') ?? '')
        if (Number.isFinite(last) && Date.now() - last < 24 * 3600 * 1000) return
        const release = await latestRelease()
        if (!release) return
        Settings.set('lastAppUpdateCheck', new Date().toISOString())
        Settings.set('latestAppVersion', release.version)
        Settings.set('latestAppPage', release.page)
        this.refresh()
    },

    /** Shows the last release found, unless it isn't newer or was dismissed. */
    refresh() {
        const version = Settings.get('latestAppVersion')
        const page = Settings.get('latestAppPage')
        this.available =
            version && page && isNewerVersion(version, AppInfo.version) && Settings.get('dismissedAppVersion') !== version
                ? { version, page }
                : null
        Hub.changed('updater')
    },

    dismiss() {
        Settings.set('dismissedAppVersion', this.available?.version ?? null)
        this.available = null
        Hub.changed('updater')
    },
}

/** The latest non-draft, non-prerelease release, or null if GitHub can't be reached. */
async function latestRelease() {
    try {
        const response = await net.fetch(`https://api.github.com/repos/${AppInfo.repository}/releases/latest`, {
            headers: { Accept: 'application/vnd.github+json' },
        })
        if (response.status !== 200) return null
        const release = await response.json()
        if (typeof release.tag_name !== 'string' || typeof release.html_url !== 'string') return null
        return { version: release.tag_name.replace(/^v/, ''), page: release.html_url }
    } catch {
        return null
    }
}

Hub.provide('updater', () => ({ available: Updater.available }))
