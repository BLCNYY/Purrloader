// A yt-dlp error turned into a short sentence plus the one action most likely to fix it.
// The original text stays in the download's log. A port of PurrloaderCore's FriendlyError.swift.

export const Fix = {
    browserSignIn: 'browserSignIn',
    updateAndRetry: 'updateAndRetry',
    chooseFolder: 'chooseFolder',
    retry: 'retry',
    useBestQuality: 'useBestQuality',
    waitForStart: 'waitForStart',
}

export const FIX_TITLES = {
    browserSignIn: 'Use Browser Sign-In',
    updateAndRetry: 'Update and Retry',
    chooseFolder: 'Choose Another Folder',
    retry: 'Retry',
    useBestQuality: 'Use Best Quality',
    waitForStart: 'Wait for Start',
}

const MESSAGES = {
    browserCookies: "Couldn't read your sign-in from that browser. Try a different browser.",
    signInRequired: 'This needs you to be signed in. Sign in on the site in your browser, then let Purrloader use it.',
    unsupportedLink: "Purrloader can't download from this link yet.",
    invalidLink: "That doesn't look like a link.",
    unavailable: "This video isn't available. It may have been removed, be private, or be blocked where you are.",
    formatUnavailable: "That quality isn't available for this video.",
    blocked: "The site blocked the download. Updating Purrloader's helper tools usually fixes this.",
    network: "Couldn't reach the site. Check your internet connection.",
    diskFull: 'Your disk is full. Free up some space, then try again.',
    folderNotWritable: "Purrloader can't save to that folder.",
    notStarted: "This stream hasn't started yet.",
    unexpectedExit: 'The download stopped unexpectedly.',
}

export function friendlyError(kind, message) {
    return { kind, message: message ?? MESSAGES[kind] ?? 'Something went wrong.' }
}

export function fixFor(error) {
    switch (error?.kind) {
        case 'browserCookies':
        case 'signInRequired':
        case 'unavailable':
            return Fix.browserSignIn
        case 'unsupportedLink':
        case 'blocked':
            return Fix.updateAndRetry
        case 'folderNotWritable':
            return Fix.chooseFolder
        case 'formatUnavailable':
            return Fix.useBestQuality
        case 'notStarted':
            return Fix.waitForStart
        case 'invalidLink':
        case undefined:
            return null
        default:
            return Fix.retry
    }
}

/** Maps a raw yt-dlp error line (without the "ERROR:" prefix). Windows and Linux phrasings of the same
 * problems sit next to the macOS ones. */
export function friendlyErrorFromRaw(raw) {
    const text = String(raw).toLowerCase()
    const has = (...needles) => needles.some((needle) => text.includes(needle))

    if (has('cookies database', 'decrypt', 'could not copy', 'failed to load cookies', 'keyring')) {
        return friendlyError('browserCookies')
    }
    if (
        has(
            'not a bot',
            'confirm your age',
            'age-restricted',
            'members-only',
            'join this channel',
            'private video',
            'login required',
            'log in',
            'sign in',
            'use --cookies',
        )
    ) {
        return friendlyError('signInRequired')
    }
    if (has('unsupported url')) return friendlyError('unsupportedLink')
    if (has('is not a valid url')) return friendlyError('invalidLink')
    if (has('video unavailable', 'this video has been removed', 'not available in your country', 'geo restrict')) {
        return friendlyError('unavailable')
    }
    if (has('requested format is not available')) return friendlyError('formatUnavailable')
    if (
        has(
            'http error 403',
            'http error 429',
            'precondition check failed',
            'nsig extraction failed',
            'signature extraction',
            'unable to extract',
        )
    ) {
        return friendlyError('blocked')
    }
    if (
        has(
            'unable to download webpage',
            'nodename nor servname',
            'timed out',
            'network is unreachable',
            'connection reset',
            'temporary failure in name resolution',
            'name or service not known',
            'getaddrinfo failed',
            'no address associated with hostname',
            'winerror 10060',
            'winerror 10054',
            'winerror 10065',
        )
    ) {
        return friendlyError('network')
    }
    if (has('no space left', 'not enough space on the disk')) return friendlyError('diskFull')
    if (has('permission denied', 'operation not permitted', 'read-only file system', 'access is denied')) {
        return friendlyError('folderNotWritable')
    }
    if (has('this live event will begin', 'premieres in', 'is_upcoming')) return friendlyError('notStarted')
    // Drop yt-dlp's "[extractor] id: " prefix.
    const match = String(raw).match(/^\[[^\]]+\] [^:]+: (.+)$/s)
    return friendlyError('other', match ? match[1] : String(raw))
}
