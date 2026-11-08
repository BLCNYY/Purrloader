// What a running yt-dlp is doing, built up from its output one line at a time.
// A port of PurrloaderCore's DownloadProgress.swift.

/** Prefixes of the progress lines Purrloader asks yt-dlp to print. */
export const PROGRESS_PREFIX = '[purrloader]'
export const POSTPROCESS_PREFIX = '[purrloaderpp]'

function number(text) {
    if (text == null || text.trim() === '') return null
    const value = Number(text)
    return Number.isFinite(value) ? value : null
}

export class DownloadProgress {
    constructor() {
        /** 'starting' | 'downloading' | 'processing' */
        this.phase = 'starting'
        /** A short plain-language description of the current step, or "" while bytes are flowing. */
        this.stage = 'Starting…'
        this.fraction = null
        this.downloadedBytes = 0
        this.totalBytes = 0
        this.speed = 0
        this.eta = null
        this.itemIndex = null
        this.itemCount = null
        /** Title yt-dlp reported for the current video; for playlists, the current item. */
        this.reportedTitle = null
        /** Files yt-dlp started writing, for cleaning up .part files on cancel. */
        this.destinations = []
        /** The last ERROR line, without its prefix. */
        this.lastError = null
        /** Bytes of streams already finished in the current item (video before audio), so progress spans the
         * whole item. */
        this.finishedStreamBytes = 0
    }

    get isPlaylist() {
        return this.itemCount != null
    }

    /** Feeds one line of output. Returns false for Purrloader's own progress lines, which don't belong in the
     * log. */
    consume(line) {
        if (line.startsWith(PROGRESS_PREFIX)) {
            this.parseProgress(line.slice(PROGRESS_PREFIX.length))
            return false
        }
        if (line.startsWith(POSTPROCESS_PREFIX)) {
            const parts = line.slice(POSTPROCESS_PREFIX.length).split('|').filter((part) => part !== '')
            if (parts[0] === 'started' && parts.length > 1) {
                this.phase = 'processing'
                this.stage = describePostprocessor(parts[1])
                this.fraction = null
            }
            return false
        }

        let match
        if (line.startsWith('ERROR:')) {
            this.lastError = line.slice(6).trim()
        } else if ((match = line.match(/Downloading item (\d+) of (\d+)/))) {
            this.itemIndex = Number(match[1])
            this.itemCount = Number(match[2])
            this.finishedStreamBytes = 0
        } else if ((match = line.match(/^\[download\] Destination: (.+)$/))) {
            this.destinations.push(match[1])
        } else if (
            line.includes('has already been downloaded') ||
            line.includes('has already been recorded in the archive')
        ) {
            this.stage = 'Already downloaded'
        } else if (line.startsWith('[Merger]')) {
            this.phase = 'processing'
            this.stage = 'Combining video and audio…'
        } else if (line.includes('Waiting for video') || line.startsWith('[wait]')) {
            this.stage = 'Waiting for the stream to start…'
        } else if (this.phase === 'starting' && line.startsWith('[')) {
            this.stage = describeExtractorLine(line)
        }
        return true
    }

    /** `status|downloaded|total|estimate|speed|eta|title`, as requested with --progress-template. */
    parseProgress(payload) {
        let parts = payload.split('|')
        if (parts.length > 7) parts = [...parts.slice(0, 6), parts.slice(6).join('|')]
        if (parts.length < 6) return

        const earlier = this.finishedStreamBytes
        const downloaded = earlier + (number(parts[1]) ?? 0)
        const streamTotal = number(parts[2]) ?? number(parts[3]) ?? 0
        const total = streamTotal > 0 ? earlier + streamTotal : 0
        this.downloadedBytes = Math.trunc(downloaded)
        this.totalBytes = Math.trunc(total)
        this.speed = number(parts[4]) ?? 0
        const eta = number(parts[5])
        this.eta = eta == null ? null : Math.trunc(eta)
        const itemFraction = total > 0 ? Math.min(1, downloaded / total) : null
        if (parts[0] === 'finished') {
            this.finishedStreamBytes += Math.trunc(streamTotal > 0 ? streamTotal : downloaded - earlier)
        }
        if (this.itemIndex != null && this.itemCount != null && this.itemCount > 1) {
            this.fraction = (this.itemIndex - 1 + (itemFraction ?? 0)) / this.itemCount
        } else {
            this.fraction = itemFraction
        }
        if (parts.length > 6 && parts[6] !== 'NA') this.reportedTitle = parts[6]
        this.phase = 'downloading'
        this.stage = parts[0] === 'finished' ? 'Finishing…' : ''
    }
}

/** What yt-dlp is doing before the first byte arrives, in plain words. */
function describeExtractorLine(line) {
    if (
        line.includes('Downloading playlist') ||
        line.includes(':tab]') ||
        line.includes('Downloading page') ||
        line.includes('playlist')
    ) {
        return 'Reading playlist…'
    }
    if (line.includes('Waiting')) return 'Waiting…'
    if (line.includes('Sleeping')) return 'Pausing between requests…'
    return 'Getting video info…'
}

function describePostprocessor(name) {
    switch (name) {
        case 'Merger':
            return 'Combining video and audio…'
        case 'ExtractAudio':
            return 'Converting audio…'
        case 'VideoRemuxer':
        case 'VideoConvertor':
            return 'Converting video…'
        case 'EmbedThumbnail':
            return 'Adding cover art…'
        case 'FFmpegMetadata':
            return 'Adding details…'
        case 'EmbedSubtitle':
            return 'Adding subtitles…'
        case 'SponsorBlock':
        case 'ModifyChapters':
            return 'Skipping sponsor segments…'
        case 'SplitChapters':
            return 'Splitting chapters…'
        case 'MoveFiles':
            return 'Finishing…'
        default:
            return 'Processing…'
    }
}
