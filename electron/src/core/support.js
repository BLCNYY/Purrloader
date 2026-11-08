// Formatting, link parsing, shell quoting and other small helpers shared by the main process and the windows.
// A port of PurrloaderCore's Support.swift, AppRelease.swift and PlaylistSelection.

export const Format = {
    /** Sizes like Finder shows them: decimal units, whole KB, one decimal for MB, two from GB up. */
    bytes(value) {
        const n = Math.trunc(Number(value) || 0)
        if (n === 0) return 'Zero KB'
        if (Math.abs(n) < 1000) return `${n} ${Math.abs(n) === 1 ? 'byte' : 'bytes'}`
        const units = ['KB', 'MB', 'GB', 'TB', 'PB']
        for (let i = 0; i < units.length; i++) {
            const digits = i === 0 ? 0 : i === 1 ? 1 : 2
            const scaled = n / 1000 ** (i + 1)
            const rounded = Number(scaled.toFixed(digits))
            if (Math.abs(rounded) < 1000 || i === units.length - 1) {
                const text = new Intl.NumberFormat(undefined, { maximumFractionDigits: digits }).format(rounded)
                return `${text} ${units[i]}`
            }
        }
        return `${n} bytes`
    },

    duration(seconds) {
        if (seconds == null || !(seconds > 0)) return null
        const s = Math.round(seconds)
        const two = (x) => String(x).padStart(2, '0')
        return s >= 3600
            ? `${Math.floor(s / 3600)}:${two(Math.floor((s % 3600) / 60))}:${two(s % 60)}`
            : `${Math.floor(s / 60)}:${two(s % 60)}`
    },

    heightLabel(height) {
        if (height >= 4320) return '8K'
        if (height >= 2160) return '4K'
        return `${height}p`
    },
}

/** http(s) links found in the text, in order. Links pasted from prose often end with punctuation that isn't
 * part of them, so that is dropped. */
export function webLinks(text) {
    const strip = /^[.,;:!?"'”’>]+|[.,;:!?"'”’>]+$/g
    return String(text ?? '')
        .split(/\s+/)
        .map((word) => word.replace(strip, ''))
        .filter((word) => {
            if (!word) return false
            try {
                const url = new URL(word)
                return (url.protocol === 'http:' || url.protocol === 'https:') && url.hostname !== ''
            } catch {
                return false
            }
        })
}

/** Compares dotted version numbers, ignoring a leading "v": "v1.10" is newer than "1.9". */
export function isNewerVersion(candidate, current) {
    const parts = (version) =>
        String(version)
            .replace(/^[vV]/, '')
            .split('.')
            .map((part) => parseInt(part.match(/^\d*/)[0] || '0', 10))
    const a = parts(candidate)
    const b = parts(current)
    for (let i = 0; i < Math.max(a.length, b.length); i++) {
        const x = a[i] ?? 0
        const y = b[i] ?? 0
        if (x !== y) return x > y
    }
    return false
}

/** "1.0.0" -> "1.0", so the version reads like the Mac app's. */
export function displayVersion(version) {
    return String(version).replace(/^(\d+\.\d+)\.0$/, '$1')
}

/** Turns ticked playlist positions into yt-dlp's -I syntax: [1,2,3,5,7,8] -> "1:3,5,7:8". */
export function compressPlaylistSelection(indices) {
    const sorted = [...new Set(indices)].sort((a, b) => a - b)
    const parts = []
    let i = 0
    while (i < sorted.length) {
        let j = i
        while (j + 1 < sorted.length && sorted[j + 1] === sorted[j] + 1) j++
        parts.push(j > i ? `${sorted[i]}:${sorted[j]}` : `${sorted[i]}`)
        i = j + 1
    }
    return parts.join(',')
}

export const Shell = {
    /** POSIX-style quoting, for showing a command someone can paste into a terminal on Linux or a Mac. */
    quote(arg) {
        if (arg !== '' && /^[\p{L}\p{N}\-_./:=+,@%]+$/u.test(arg)) return arg
        return "'" + arg.replaceAll("'", "'\\''") + "'"
    },

    /** Windows command-line quoting (what CreateProcess and PowerShell both read back the same way). */
    quoteWindows(arg) {
        if (arg !== '' && /^[\p{L}\p{N}\-_./:=+,@%\\]+$/u.test(arg)) return arg
        return '"' + arg.replace(/(\\*)"/g, '$1$1\\"').replace(/(\\+)$/, '$1$1') + '"'
    },

    commandLine(args, platform = 'linux') {
        const quote = platform === 'win32' ? Shell.quoteWindows : Shell.quote
        return args.map(quote).join(' ')
    },

    /** Splits a string into arguments the way a POSIX shell would (quotes and backslashes, no expansion). */
    split(text) {
        const args = []
        let current = ''
        let hasToken = false
        let quote = null
        let escaped = false
        for (const ch of String(text ?? '')) {
            if (escaped) {
                current += ch
                escaped = false
                hasToken = true
            } else if (ch === '\\' && quote !== "'") {
                escaped = true
            } else if (quote) {
                if (ch === quote) quote = null
                else current += ch
            } else if (ch === "'" || ch === '"') {
                quote = ch
                hasToken = true
            } else if (/\s/.test(ch)) {
                if (hasToken) {
                    args.push(current)
                    current = ''
                    hasToken = false
                }
            } else {
                current += ch
                hasToken = true
            }
        }
        if (hasToken) args.push(current)
        return args
    },
}

/** Finds the SHA-256 for `fileName` in a checksum file: `<hash>  <name>` lines (sha256sum style, with an
 * optional `*` before binary names), or a file holding just the hash. */
export function checksumFor(fileName, text) {
    const lines = String(text)
        .split(/\r?\n|\r/)
        .map((line) => line.trim().split(/\s+/).filter(Boolean))
        .filter((fields) => fields.length > 0)
    const isHash = (s) => /^[0-9a-fA-F]{64}$/.test(s)
    for (const fields of lines) {
        if (fields.length >= 2 && isHash(fields[0])) {
            const name = fields[1].startsWith('*') ? fields[1].slice(1) : fields[1]
            if (name === fileName) return fields[0].toLowerCase()
        }
    }
    if (lines.length === 1 && lines[0].length === 1 && isHash(lines[0][0])) return lines[0][0].toLowerCase()
    return null
}
