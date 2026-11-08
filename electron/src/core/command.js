// Turns download options into yt-dlp arguments (without the executable itself).
// A port of PurrloaderCore's YtDlpCommand.swift.

import { POSTPROCESS_PREFIX, PROGRESS_PREFIX } from './progress.js'
import { DEFAULT_TEMPLATE, supportsChapters, supportsCoverArt } from './options.js'
import { Shell } from './support.js'

/**
 * @typedef {object} CommandEnvironment Paths the arguments refer to, passed in so building a command never
 * touches the disk.
 * @property {string} ffmpeg
 * @property {string} deno
 * @property {string} archive
 * @property {string} [aria2c] aria2c, if the person asked for it and it's installed.
 * @property {boolean} [playsAV1] Whether this computer decodes AV1 in hardware.
 * @property {string} [platform] 'win32', 'linux' or 'darwin'.
 * @property {string} [home] The home folder, for "~" in a cookies file path.
 */

export class YtDlpCommand {
    /** @param {CommandEnvironment} environment */
    constructor(options, environment) {
        this.options = options
        this.environment = { playsAV1: true, platform: 'linux', home: '', ...environment }
    }

    /** A download. `playlistItems` (yt-dlp -I syntax) overrides the playlist settings; `filepathLog` receives
     * the final path of every file. */
    download(url, folder, { playlistItems = null, filepathLog = null } = {}) {
        const a = [...this.access(), ...this.format()]
        if (playlistItems) {
            a.push('--yes-playlist', '-I', playlistItems)
            if (this.options.playlist.useArchive) a.push('--download-archive', this.environment.archive)
        } else {
            a.push(...this.playlist())
        }
        a.push(...this.subtitles(), ...this.extras(), ...this.transfer(), ...this.output(folder))
        a.push(
            '--newline',
            '--progress-delta',
            '0.5',
            '--progress-template',
            `download:${PROGRESS_PREFIX}%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|` +
                '%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.title)s',
            '--progress-template',
            `postprocess:${POSTPROCESS_PREFIX}%(progress.status)s|%(progress.postprocessor)s`,
        )
        if (filepathLog) a.push('--print-to-file', 'after_move:filepath', filepathLog)
        a.push(...Shell.split(this.options.extraArguments))
        a.push('--', url)
        return a
    }

    /** Reads a link's details as JSON, without the entries of a playlist. */
    info(url) {
        return [...this.access(), '-J', '--flat-playlist', '--no-warnings', '--', url]
    }

    /** Shared by info and download: config, tools, sign-in and network. */
    access() {
        const network = this.options.network
        const env = this.environment
        const a = []
        if (this.options.ignoreConfig) a.push('--ignore-config')
        a.push('--ffmpeg-location', env.ffmpeg, '--js-runtimes', `deno:${env.deno}`)

        const profile = network.cookieProfile.trim()
        if (network.cookieBrowser !== 'none') {
            let spec
            if (network.cookieBrowser === 'arc' && env.platform === 'darwin') {
                // yt-dlp has no Arc entry; Arc is Chromium-based, so point it at Arc's profile.
                spec = `chromium:${env.home}/Library/Application Support/Arc/User Data/${profile || 'Default'}`
            } else {
                spec = network.cookieBrowser === 'arc' ? 'chromium' : network.cookieBrowser
                if (profile) spec += ':' + profile
            }
            a.push('--cookies-from-browser', spec)
        } else if (network.cookiesFile.trim()) {
            a.push('--cookies', expandTilde(network.cookiesFile.trim(), env.home))
        }
        if (network.proxy.trim()) a.push('--proxy', network.proxy.trim())
        if (network.impersonate.trim()) a.push('--impersonate', network.impersonate.trim())
        if (network.xff.trim()) a.push('--xff', network.xff.trim())
        if (network.userAgent.trim()) a.push('--add-headers', `User-Agent:${network.userAgent.trim()}`)
        return a
    }

    format() {
        const format = this.options.format
        const exact = format.exactFormat.trim()
        const a = []
        if (format.mode === 'audio') {
            a.push('-f', exact || 'ba/b', '-x')
            if (format.audioFormat !== 'best') {
                a.push('--audio-format', format.audioFormat)
                if (['mp3', 'm4a', 'opus'].includes(format.audioFormat)) a.push('--audio-quality', format.audioQuality)
            }
            return a
        }

        const limit = format.maxHeight != null ? `[height<=${format.maxHeight}]` : ''
        a.push('-f', exact || `bv*${limit}+ba/b${limit}`)
        const sort = []
        switch (format.codec) {
            // Compatibility first: H.264 wins even over resolution.
            case 'h264':
                sort.push('vcodec:h264', 'res', 'acodec:m4a')
                break
            case 'hevc':
                sort.push('res', 'vcodec:h265')
                break
            case 'vp9':
                sort.push('res', 'vcodec:vp9')
                break
            case 'av1':
                sort.push('res', 'vcodec:av01')
                break
        }
        if (format.container === 'mp4') {
            // Prefer streams that are already MP4 (H.264/AV1 + AAC) so the result merges into an MP4 that the
            // built-in players play, without re-encoding. MKV/Automatic keep the very best streams. Computers
            // without an AV1 decoder get H.264 instead, even if that means a lower resolution.
            if (format.codec === 'any') {
                sort.unshift(...(this.environment.playsAV1 ? ['ext:mp4:m4a'] : ['vcodec:h264', 'ext:mp4:m4a']))
            } else if (!sort.includes('acodec:m4a')) {
                sort.push('acodec:m4a')
            }
        }
        if (!format.allowHDR) sort.unshift('hdr:sdr')
        if (sort.length) a.push('-S', sort.join(','))
        if (format.container !== 'auto') {
            a.push('--merge-output-format', format.container)
            a.push(format.containerMethod === 'remux' ? '--remux-video' : '--recode-video', format.container)
        }
        return a
    }

    playlist() {
        const playlist = this.options.playlist
        const a = [playlist.noPlaylist ? '--no-playlist' : '--yes-playlist']
        const range = playlist.range.trim()
        if (playlist.latestCount > 0) {
            a.push('-I', playlist.reverse ? `${playlist.latestCount}:1:-1` : `1:${playlist.latestCount}`)
        } else if (range) {
            const bounds = range.split(/-(.*)/s).slice(0, 2).map((part) => part.trim())
            const lo = /^[+-]?\d+$/.test(bounds[0] ?? '') ? Number(bounds[0]) : null
            const hi = /^[+-]?\d+$/.test(bounds[1] ?? '') ? Number(bounds[1]) : null
            if (bounds.length === 2 && lo != null && hi != null) {
                a.push('-I', playlist.reverse ? `${hi}:${lo}:-1` : `${lo}:${hi}`)
            } else {
                a.push('-I', range)
            }
        } else if (playlist.reverse) {
            a.push('-I', '::-1')
        }
        if (playlist.useArchive) a.push('--download-archive', this.environment.archive)
        return a
    }

    subtitles() {
        const subtitles = this.options.subtitles
        if (!subtitles.enabled) return []
        const a = ['--write-subs']
        if (subtitles.includeAutoGenerated) a.push('--write-auto-subs')
        a.push('--sub-langs', subtitles.languages.trim() || 'en.*')
        if (subtitles.convertToSRT) a.push('--convert-subs', 'srt')
        if (subtitles.embed && this.options.format.mode === 'video') a.push('--embed-subs')
        return a
    }

    extras() {
        const extras = this.options.extras
        const format = this.options.format
        const a = []
        if (extras.writeThumbnail) a.push('--write-thumbnail')
        if (extras.embedThumbnail && supportsCoverArt(format)) a.push('--embed-thumbnail')
        if (extras.embedMetadata) a.push('--embed-metadata')
        if (extras.embedChapters && supportsChapters(format)) a.push('--embed-chapters')
        if (extras.splitChapters) a.push('--split-chapters')

        if (extras.sponsorBlock !== 'off' && extras.sponsorCategories.length) {
            a.push(
                extras.sponsorBlock === 'mark' ? '--sponsorblock-mark' : '--sponsorblock-remove',
                extras.sponsorCategories.join(','),
            )
        }

        const start = extras.sectionStart.trim()
        const end = extras.sectionEnd.trim()
        if (start || end) a.push('--download-sections', `*${start || '0'}-${end || 'inf'}`)
        if (extras.chapterRegex.trim()) a.push('--download-sections', extras.chapterRegex.trim())
        if (extras.forceKeyframes) a.push('--force-keyframes-at-cuts')

        if (extras.liveFromStart) a.push('--live-from-start')
        if (extras.waitForVideo) a.push('--wait-for-video', `${Math.max(1, extras.waitInterval)}`)

        if (extras.writeDescription) a.push('--write-description')
        if (extras.writeComments) a.push('--write-comments')
        if (extras.writeInfoJSON) a.push('--write-info-json')
        return a
    }

    transfer() {
        const network = this.options.network
        const a = []
        if (network.rateLimit.trim()) a.push('-r', network.rateLimit.trim())
        if (network.retries !== 10) a.push('-R', `${network.retries}`)
        if (network.concurrentFragments > 1) a.push('-N', `${network.concurrentFragments}`)
        if (network.useAria2c && this.environment.aria2c) a.push('--downloader', this.environment.aria2c)
        return a
    }

    output(folder) {
        const output = this.options.output
        const template = output.template.trim() || DEFAULT_TEMPLATE
        const a = ['-P', folder, '-o', template]
        if (output.restrictFilenames) {
            a.push('--restrict-filenames')
        } else {
            // Windows needs yt-dlp's Windows-safe names (its default there). Elsewhere keep characters the
            // system allows ("?", "|", "*") instead of full-width look-alikes.
            if (this.environment.platform !== 'win32') a.push('--no-windows-filenames')
            // ": " in titles reads better as " - " than as a look-alike colon, and the Mac shows ":" as "/".
            for (const [pattern, replacement] of [
                ['\\s*:\\s+', ' - '],
                [':', '-'],
            ]) {
                a.push('--replace-in-metadata', 'title,playlist_title,uploader,channel', pattern, replacement)
            }
        }
        switch (output.overwrite) {
            case 'never':
                a.push('--no-overwrites')
                break
            case 'force':
                a.push('--force-overwrites')
                break
        }
        return a
    }
}

function expandTilde(path, home) {
    if (!home) return path
    if (path === '~') return home
    if (path.startsWith('~/') || path.startsWith('~\\')) return home + path.slice(1)
    return path
}
