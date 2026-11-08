// The parts of yt-dlp's `-J --flat-playlist` output the windows need, and the file name template preview.
// A port of PurrloaderCore's MediaInfo.swift.

const str = (value) => (typeof value === 'string' ? value : null)
const num = (value) => (typeof value === 'number' && Number.isFinite(value) ? value : null)
const int = (value) => (Number.isInteger(value) ? value : null)

/** The language name for a code like "de" or "pt-BR", in the reader's language. */
export function languageName(code, locale) {
    try {
        return new Intl.DisplayNames([locale || 'en'], { type: 'language' }).of(code) || code
    } catch {
        return code
    }
}

function subtitleTitle(code, isAutomatic, locale) {
    const language = code.endsWith('-orig') ? code.slice(0, -5) : code
    const name = languageName(language, locale)
    return isAutomatic ? `${name} (auto-generated)` : name
}

export function subtitleTrack(code, isAutomatic, locale) {
    return { code, isAutomatic, title: subtitleTitle(code, isAutomatic, locale) }
}

function formatResolution(f) {
    if (!f.hasVideo) return 'audio'
    if (f.height == null) return 'video'
    return `${f.height}p` + (f.fps != null && f.fps > 30 ? `${Math.trunc(f.fps)}` : '')
}

/** @param {string} locale e.g. "en-US"; decides language names and which auto-generated subtitles to offer. */
export function parseMediaInfo(json, url, locale = 'en') {
    const language = String(locale).split(/[-_]/)[0].toLowerCase()
    const entriesJSON = Array.isArray(json.entries) ? json.entries.filter((e) => e && typeof e === 'object') : []

    let thumbnail = str(json.thumbnail)
    if (!thumbnail && Array.isArray(json.thumbnails) && json.thumbnails.length) {
        thumbnail = str(json.thumbnails.at(-1)?.url)
    }
    if (!thumbnail && entriesJSON.length && Array.isArray(entriesJSON[0].thumbnails)) {
        thumbnail = str(entriesJSON[0].thumbnails.at(-1)?.url)
    }

    const entries = entriesJSON.map((e, offset) => ({
        // 1-based position in the playlist, what -I expects.
        index: offset + 1,
        id: str(e.id) ?? `${offset}`,
        title: str(e.title) ?? str(e.url) ?? `Item ${offset + 1}`,
        duration: num(e.duration),
    }))

    const formats = (Array.isArray(json.formats) ? json.formats : [])
        .map((f) => {
            const id = str(f?.format_id)
            if (!id) return null
            const vcodec = str(f.vcodec)
            const acodec = str(f.acodec)
            const format = {
                id,
                ext: str(f.ext) ?? '',
                height: int(f.height),
                fps: num(f.fps),
                vcodec,
                acodec,
                bitrate: num(f.tbr),
                size: num(f.filesize) ?? num(f.filesize_approx),
                note: str(f.format_note) ?? '',
                dynamicRange: str(f.dynamic_range),
                hasVideo: vcodec != null && vcodec !== 'none',
                hasAudio: acodec != null && acodec !== 'none',
            }
            format.resolution = formatResolution(format)
            return format
        })
        .filter((f) => f && (f.hasVideo || f.hasAudio))
        .reverse() // yt-dlp lists worst first

    const heights = [...new Set(formats.filter((f) => f.hasVideo && f.height != null).map((f) => f.height))]
    heights.sort((a, b) => b - a)

    return {
        url,
        title: str(json.title) ?? url,
        uploader: str(json.uploader) ?? str(json.channel) ?? str(json.playlist_uploader),
        duration: num(json.duration),
        thumbnail,
        isLive: json.is_live === true || json.live_status === 'is_upcoming',
        isPlaylist: json._type === 'playlist',
        entries,
        formats,
        // Distinct video heights actually on offer, best first.
        availableHeights: heights,
        subtitleTracks: subtitleTracks(json.subtitles, json.automatic_captions, language, locale),
    }
}

/** Real subtitles, plus the auto-generated ones worth offering: the original spoken language and the reader's
 * own language or English. Sites can list a hundred machine translations otherwise. */
function subtitleTracks(manualJSON, automaticJSON, language, locale) {
    const manual = new Set(Object.keys(manualJSON && typeof manualJSON === 'object' ? manualJSON : {}))
    manual.delete('live_chat')
    const automatic = new Set(Object.keys(automaticJSON && typeof automaticJSON === 'object' ? automaticJSON : {}))
    const wanted = new Set([language, 'en'])
    const tracks = [...manual].map((code) => subtitleTrack(code, false, locale))
    for (const key of automatic) {
        let code = null
        if (key.endsWith('-orig')) {
            const base = key.slice(0, -5)
            code = automatic.has(base) ? base : key
        } else if (wanted.has(key)) {
            code = key
        }
        if (code && !tracks.some((track) => track.code === code)) tracks.push(subtitleTrack(code, true, locale))
    }
    return tracks.sort((a, b) => {
        const order = Number(a.isAutomatic) - Number(b.isAutomatic)
        if (order) return order
        return a.title < b.title ? -1 : a.title > b.title ? 1 : 0
    })
}

/** The saved choice if this video has it, else the system language, then English, then anything. */
export function defaultSubtitleCode(info, saved, locale = 'en') {
    const codes = info.subtitleTracks.map((track) => track.code)
    if (saved && codes.includes(saved)) return saved
    const language = String(locale).split(/[-_]/)[0].toLowerCase()
    for (const wanted of [...new Set([language, 'en'])]) {
        const match = codes.find((code) => code === wanted || code.startsWith(wanted + '-'))
        if (match) return match
    }
    return codes[0] ?? null
}

// MARK: - File name preview

export const TEMPLATE_PRESETS = [
    ['Title', '%(title)s.%(ext)s'],
    ['Title [ID]', '%(title)s [%(id)s].%(ext)s'],
    ['Uploader / Title', '%(uploader)s/%(title)s.%(ext)s'],
    ['Date - Title', '%(upload_date>%Y-%m-%d)s - %(title)s.%(ext)s'],
    ['Playlist / 001 - Title', '%(playlist_title,playlist|NA)s/%(playlist_index)03d - %(title)s.%(ext)s'],
    ['Channel / Date - Title [ID]', '%(channel,uploader)s/%(upload_date>%Y-%m-%d)s - %(title)s [%(id)s].%(ext)s'],
]

const SAMPLE = {
    id: 'a1b2c3d4e5f',
    title: 'Example Video Title',
    uploader: 'Example Channel',
    channel: 'Example Channel',
    upload_date: '20240131',
    playlist_title: 'Example Playlist',
    playlist_index: 1,
    duration: 212,
    height: 1080,
    resolution: '1920x1080',
}

const isScalar = (value) => value === null || typeof value !== 'object'

/** Approximates yt-dlp's output template rendering for the live file name preview. */
export function renderTemplate(template, info, { ext, restrict = false, platform = 'linux', locale = 'en' } = {}) {
    const fields = { ...SAMPLE }
    if (info) {
        for (const [key, value] of Object.entries(info)) if (isScalar(value)) fields[key] = value
        if (info._type === 'playlist') {
            fields.playlist_title = info.title
            const first = Array.isArray(info.entries) ? info.entries[0] : null
            if (first && typeof first === 'object') {
                for (const [key, value] of Object.entries(first)) if (isScalar(value)) fields[key] = value
            }
        }
    }
    fields.ext = ext

    const pattern = /%\(([^)]+)\)([-#0+ ]*\d*(?:\.\d+)?)([diouxXeEfFgGcrsaBjlqDSU])/g
    return template.replace(pattern, (_, expr, flags, conversion) =>
        sanitize(fieldValue(expr, fields, flags, conversion, locale), restrict, platform),
    )
}

function fieldValue(expression, fields, flags, conversion, locale) {
    let expr = expression
    let fallback = 'NA'
    const bar = expr.indexOf('|')
    if (bar >= 0) {
        fallback = expr.slice(bar + 1)
        expr = expr.slice(0, bar)
    }
    let dateFormat = null
    const gt = expr.indexOf('>')
    if (gt >= 0) {
        dateFormat = expr.slice(gt + 1)
        expr = expr.slice(0, gt)
    }
    // "a,b" means first available of a or b
    const raw = expr
        .split(',')
        .map((key) => fields[key.trim()])
        .find((value) => value !== undefined)
    if (raw === undefined || raw === null) return fallback

    if (dateFormat && typeof raw === 'string' && /^\d{8}$/.test(raw)) return strftime(dateFormat, raw, locale)
    if (conversion === 'd' && typeof raw === 'number') {
        const n = Math.trunc(raw)
        const width = parseInt(flags.replace(/\D/g, '') || '0', 10)
        return flags.startsWith('0') ? String(n).padStart(width, '0') : String(n)
    }
    return String(raw)
}

function strftime(format, yyyymmdd, locale) {
    const year = yyyymmdd.slice(0, 4)
    const month = yyyymmdd.slice(4, 6)
    const day = yyyymmdd.slice(6, 8)
    const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)))
    const monthName = (style) => new Intl.DateTimeFormat(locale, { month: style, timeZone: 'UTC' }).format(date)
    const map = {
        '%Y': year,
        '%m': month,
        '%d': day,
        '%y': year.slice(2),
        '%H': '00',
        '%M': '00',
        '%S': '00',
        '%b': monthName('short'),
        '%B': monthName('long'),
    }
    return format.replace(/%[YmdyHMSbB]/g, (token) => map[token])
}

/** What yt-dlp does on Windows to characters Windows doesn't allow in file names. */
const WINDOWS_LOOKALIKES = { '\\': '⧹', '|': '｜', '*': '＊', '?': '？', '<': '＜', '>': '＞', '"': '＂' }

function sanitize(value, restrict, platform) {
    // Mirrors the output arguments: ": " becomes " - ", a bare ":" becomes "-".
    let out = value
        .replaceAll('/', '⧸')
        .replace(/\s*:\s+/g, ' - ')
        .replaceAll(':', '-')
    if (restrict) {
        out = out.normalize('NFKD').replace(/\p{M}/gu, '')
        out = [...out].map((ch) => (/^[A-Za-z0-9\-_.[\]()]$/.test(ch) ? ch : '_')).join('')
    } else if (platform === 'win32') {
        out = out.replace(/[\\|*?<>"]/g, (ch) => WINDOWS_LOOKALIKES[ch])
    }
    return out
}
