// Ports of Tests/PurrloaderCoreTests, so the Windows and Linux app builds the same commands as the Mac app.
import assert from 'node:assert/strict'
import { describe, test } from 'node:test'

import { YtDlpCommand } from '../src/core/command.js'
import { Fix, fixFor, friendlyError, friendlyErrorFromRaw } from '../src/core/errors.js'
import { defaultSubtitleCode, parseMediaInfo, renderTemplate } from '../src/core/media.js'
import { decodeOptions, defaultOptions } from '../src/core/options.js'
import { DownloadProgress } from '../src/core/progress.js'
import {
    Format,
    Shell,
    checksumFor,
    compressPlaylistSelection,
    displayVersion,
    isNewerVersion,
    webLinks,
} from '../src/core/support.js'

/** True if `sequence` appears in `array` as consecutive elements. */
function containsSequence(array, sequence) {
    for (let i = 0; i + sequence.length <= array.length; i++) {
        if (sequence.every((item, j) => array[i + j] === item)) return true
    }
    return false
}

describe('YtDlpCommand', () => {
    const environment = {
        ffmpeg: '/tools/ffmpeg',
        deno: '/tools/deno',
        archive: '/data/archive.txt',
        platform: 'darwin',
        home: '/Users/me',
    }
    const folder = '/Users/me/Downloads'
    const url = 'https://example.com/watch/123'

    function args(configure = () => {}, { playlistItems = null, env = environment } = {}) {
        const options = defaultOptions()
        configure(options)
        return new YtDlpCommand(options, env).download(url, folder, { playlistItems })
    }

    test('default video is a player-friendly MP4', () => {
        const a = args()
        assert.ok(containsSequence(a, ['-f', 'bv*+ba/b']))
        assert.ok(containsSequence(a, ['-S', 'ext:mp4:m4a']))
        assert.ok(containsSequence(a, ['--merge-output-format', 'mp4']))
        assert.ok(containsSequence(a, ['--remux-video', 'mp4']))
        for (const flag of ['--embed-thumbnail', '--embed-metadata', '--embed-chapters', '--ignore-config']) {
            assert.ok(a.includes(flag), flag)
        }
        assert.ok(containsSequence(a, ['--ffmpeg-location', '/tools/ffmpeg']))
        assert.ok(containsSequence(a, ['--js-runtimes', 'deno:/tools/deno']))
        assert.ok(containsSequence(a, ['-P', '/Users/me/Downloads', '-o', '%(title)s.%(ext)s']))
        assert.ok(a.includes('--no-windows-filenames'))
        assert.deepEqual(a.slice(-2), ['--', url])
    })

    test('Windows keeps yt-dlp’s Windows-safe file names', () => {
        const a = args(() => {}, { env: { ...environment, platform: 'win32' } })
        assert.ok(!a.includes('--no-windows-filenames'))
        assert.ok(a.includes('--replace-in-metadata'))
        assert.ok(args(() => {}, { env: { ...environment, platform: 'linux' } }).includes('--no-windows-filenames'))
    })

    test('computers without AV1 playback get H.264', () => {
        const options = defaultOptions()
        options.format.maxHeight = 2160
        const a = new YtDlpCommand(options, { ...environment, playsAV1: false }).download(url, folder)
        assert.ok(containsSequence(a, ['-S', 'vcodec:h264,ext:mp4:m4a']))
    })

    test('quality limit applies to both fallbacks', () => {
        assert.ok(containsSequence(args((o) => (o.format.maxHeight = 1080)), ['-f', 'bv*[height<=1080]+ba/b[height<=1080]']))
    })

    test('H.264 preference ranks codec first', () => {
        assert.ok(containsSequence(args((o) => (o.format.codec = 'h264')), ['-S', 'vcodec:h264,res,acodec:m4a']))
    })

    test('MP3 uses best VBR', () => {
        const a = args((o) => (o.format.mode = 'audio'))
        assert.ok(containsSequence(a, ['-f', 'ba/b', '-x', '--audio-format', 'mp3', '--audio-quality', '0']))
        assert.ok(a.includes('--embed-thumbnail'))
        assert.ok(!a.includes('--merge-output-format'))
    })

    test('best quality audio is not converted', () => {
        const a = args((o) => {
            o.format.mode = 'audio'
            o.format.audioFormat = 'best'
        })
        assert.ok(!a.includes('--audio-format'))
        assert.ok(a.includes('-x'))
    })

    test('cover art and chapters are skipped where unsupported', () => {
        const wav = args((o) => {
            o.format.mode = 'audio'
            o.format.audioFormat = 'wav'
        })
        assert.ok(!wav.includes('--embed-thumbnail'))
        assert.ok(!wav.includes('--embed-chapters'))
        assert.ok(!args((o) => (o.format.container = 'webm')).includes('--embed-thumbnail'))
    })

    test('subtitles embed in video only', () => {
        const video = args((o) => {
            o.subtitles.enabled = true
            o.subtitles.languages = 'de'
        })
        assert.ok(containsSequence(video, ['--write-subs', '--write-auto-subs', '--sub-langs', 'de']))
        assert.ok(video.includes('--embed-subs'))
        const audio = args((o) => {
            o.format.mode = 'audio'
            o.subtitles.enabled = true
        })
        assert.ok(!audio.includes('--embed-subs'))
        assert.ok(!args().includes('--write-subs'))
    })

    test('skip sponsors and part of a video', () => {
        const a = args((o) => {
            o.extras.sponsorBlock = 'remove'
            o.extras.sectionStart = '1:30'
        })
        assert.ok(containsSequence(a, ['--sponsorblock-remove', 'sponsor,selfpromo,interaction']))
        assert.ok(containsSequence(a, ['--download-sections', '*1:30-inf']))
    })

    test('browser sign-in', () => {
        assert.ok(containsSequence(args((o) => (o.network.cookieBrowser = 'firefox')), ['--cookies-from-browser', 'firefox']))
        const chrome = args((o) => {
            o.network.cookieBrowser = 'chrome'
            o.network.cookieProfile = 'Profile 1'
        })
        assert.ok(containsSequence(chrome, ['--cookies-from-browser', 'chrome:Profile 1']))
        const arc = args((o) => (o.network.cookieBrowser = 'arc'))
        assert.ok(arc.some((arg) => arg.startsWith('chromium:') && arg.endsWith('/Arc/User Data/Default')))
    })

    test('a cookies file path can start with ~', () => {
        const a = args((o) => (o.network.cookiesFile = '~/cookies.txt'))
        assert.ok(containsSequence(a, ['--cookies', '/Users/me/cookies.txt']))
    })

    test('ticked playlist items override the range', () => {
        const a = args((o) => (o.playlist.range = '5-20'), { playlistItems: '1:3,7' })
        assert.ok(containsSequence(a, ['--yes-playlist', '-I', '1:3,7']))
        assert.ok(!a.includes('5:20'))
    })

    test('playlist range and reverse', () => {
        assert.ok(containsSequence(args((o) => (o.playlist.range = '5-20')), ['-I', '5:20']))
        const reversed = args((o) => {
            o.playlist.range = '5-20'
            o.playlist.reverse = true
        })
        assert.ok(containsSequence(reversed, ['-I', '20:5:-1']))
        assert.ok(containsSequence(args((o) => (o.playlist.latestCount = 3)), ['-I', '1:3']))
        assert.ok(containsSequence(args((o) => (o.playlist.range = '1,3,7:10')), ['-I', '1,3,7:10']))
        assert.ok(
            containsSequence(args((o) => (o.playlist.useArchive = true)), ['--download-archive', '/data/archive.txt']),
        )
    })

    test('extra arguments go last before the URL', () => {
        const a = args((o) => (o.extraArguments = '--match-filters "duration < 600"'))
        assert.deepEqual(a.slice(-4), ['--match-filters', 'duration < 600', '--', url])
    })

    test('info fetch uses the same access options', () => {
        const options = defaultOptions()
        options.network.proxy = 'socks5://127.0.0.1:1080'
        const a = new YtDlpCommand(options, environment).info(url)
        assert.ok(containsSequence(a, ['--proxy', 'socks5://127.0.0.1:1080']))
        assert.deepEqual(a.slice(-5), ['-J', '--flat-playlist', '--no-warnings', '--', url])
    })
})

describe('DownloadProgress', () => {
    test('progress lines update bytes and stay out of the log', () => {
        const progress = new DownloadProgress()
        assert.equal(progress.consume('[purrloader]downloading|1048576|4194304|NA|524288.0|6|Big Buck Bunny'), false)
        assert.equal(progress.phase, 'downloading')
        assert.equal(progress.downloadedBytes, 1_048_576)
        assert.equal(progress.totalBytes, 4_194_304)
        assert.equal(progress.fraction, 0.25)
        assert.equal(progress.speed, 524_288)
        assert.equal(progress.eta, 6)
        assert.equal(progress.reportedTitle, 'Big Buck Bunny')
        assert.equal(progress.stage, '')
    })

    test('estimated total is used when the exact one is missing', () => {
        const progress = new DownloadProgress()
        progress.consume('[purrloader]downloading|500|NA|1000|NA|NA|NA')
        assert.equal(progress.fraction, 0.5)
        assert.equal(progress.eta, null)
        assert.equal(progress.reportedTitle, null)
    })

    test('titles with a bar in them survive', () => {
        const progress = new DownloadProgress()
        progress.consume('[purrloader]downloading|1|2|NA|NA|NA|Live | Part 2')
        assert.equal(progress.reportedTitle, 'Live | Part 2')
    })

    test('video then audio count as one item', () => {
        const progress = new DownloadProgress()
        progress.consume('[purrloader]finished|800|800|NA|NA|NA|Clip')
        progress.consume('[purrloader]downloading|100|200|NA|NA|NA|Clip')
        assert.equal(progress.downloadedBytes, 900)
        assert.equal(progress.totalBytes, 1000)
        assert.equal(progress.fraction, 0.9)
    })

    test('playlist progress spans all items', () => {
        const progress = new DownloadProgress()
        assert.equal(progress.consume('[download] Downloading item 2 of 4'), true)
        progress.consume('[purrloader]downloading|50|100|NA|NA|NA|Second')
        assert.ok(progress.isPlaylist)
        assert.equal(progress.fraction, 0.375)
    })

    test('postprocessing is described plainly', () => {
        const progress = new DownloadProgress()
        assert.equal(progress.consume('[purrloaderpp]started|Merger'), false)
        assert.equal(progress.phase, 'processing')
        assert.equal(progress.stage, 'Combining video and audio…')
        progress.consume('[purrloaderpp]started|SponsorBlock')
        assert.equal(progress.stage, 'Skipping sponsor segments…')
    })

    test('errors and destinations are remembered', () => {
        const progress = new DownloadProgress()
        assert.equal(progress.consume('[download] Destination: /tmp/Clip.f137.mp4'), true)
        assert.equal(progress.consume('ERROR: [site] abc: Video unavailable'), true)
        assert.deepEqual(progress.destinations, ['/tmp/Clip.f137.mp4'])
        assert.equal(progress.lastError, '[site] abc: Video unavailable')
    })

    test('stage before the first byte', () => {
        const progress = new DownloadProgress()
        progress.consume('[youtube:tab] Extracting URL: https://example.com/playlist')
        assert.equal(progress.stage, 'Reading playlist…')
    })
})

describe('support', () => {
    test('playlist selection compresses runs', () => {
        for (const [indices, expected] of [
            [[1, 2, 3, 5, 7, 8], '1:3,5,7:8'],
            [[4], '4'],
            [[], ''],
            [[9, 1, 2, 2, 10], '1:2,9:10'],
            [[1, 3, 5], '1,3,5'],
        ]) {
            assert.equal(compressPlaylistSelection(indices), expected)
        }
    })

    test('shell split works like a shell', () => {
        for (const [text, expected] of [
            ['--a b', ['--a', 'b']],
            ['  spaced   out  ', ['spaced', 'out']],
            ['--match-filters "duration < 600"', ['--match-filters', 'duration < 600']],
            ['\'single quoted\' "double"', ['single quoted', 'double']],
            ["it\\'s escaped\\ space", ["it's", 'escaped space']],
            ["'no \\escape in single'", ['no \\escape in single']],
            ['a"b"c', ['abc']],
            ["''", ['']],
            ['', []],
        ]) {
            assert.deepEqual(Shell.split(text), expected, text)
        }
    })

    test('quoting round-trips', () => {
        const args = ['plain', 'with space', "it's", '']
        assert.deepEqual(Shell.split(Shell.commandLine(args)), args)
        assert.equal(
            Shell.commandLine(['C:\\Tools\\yt-dlp.exe', 'with space', 'say "hi"'], 'win32'),
            'C:\\Tools\\yt-dlp.exe "with space" "say \\"hi\\""',
        )
    })

    test('checksums', () => {
        const sums = [
            '0f922c15ccc0b83eefa1c8bc9104e434318c4bf16e408e1ee93b7745a6efe0aa  yt-dlp_macos',
            '20d98fa608998eff87d1d35a6e9fb65e4b874639a6cc89addc19da3becde3ef1  yt-dlp_macos.zip',
        ].join('\n')
        assert.equal(checksumFor('yt-dlp_macos.zip', sums), '20d98fa608998eff87d1d35a6e9fb65e4b874639a6cc89addc19da3becde3ef1')
        assert.equal(checksumFor('missing.zip', sums), null)
        const hash = 'ab'.repeat(32)
        assert.equal(checksumFor('deno.zip', `${hash} *deno.zip\n`), hash)
        assert.equal(checksumFor('anything.zip', hash.toUpperCase() + '\n'), hash)
    })

    test('versions', () => {
        for (const [candidate, current, newer] of [
            ['1.1', '1.0', true],
            ['v1.10', '1.9', true],
            ['1.0.1', '1.0', true],
            ['1.0', '1.0', false],
            ['1.0', '1.0.0', false],
            ['0.9', '1.0', false],
        ]) {
            assert.equal(isNewerVersion(candidate, current), newer, `${candidate} vs ${current}`)
        }
        assert.equal(displayVersion('1.0.0'), '1.0')
        assert.equal(displayVersion('1.2.3'), '1.2.3')
    })

    test('web links drop trailing punctuation', () => {
        assert.deepEqual(webLinks('Watch https://example.com/v/1. Thanks!'), ['https://example.com/v/1'])
        assert.deepEqual(webLinks('ftp://nope and https://ok.example'), ['https://ok.example'])
    })

    test('formatting', () => {
        assert.equal(Format.duration(212), '3:32')
        assert.equal(Format.duration(3725), '1:02:05')
        assert.equal(Format.duration(0), null)
        assert.equal(Format.heightLabel(2160), '4K')
        assert.equal(Format.heightLabel(720), '720p')
        assert.equal(Format.bytes(0), 'Zero KB')
        assert.equal(Format.bytes(512), '512 bytes')
        assert.match(Format.bytes(4_194_304), /^4[.,]2 MB$/)
        assert.equal(Format.bytes(999_600), '1 MB')
    })
})

describe('options', () => {
    test('missing keys keep their defaults', () => {
        const options = decodeOptions({ format: { mode: 'audio' }, network: { retries: 3 } })
        assert.equal(options.format.mode, 'audio')
        assert.equal(options.format.audioFormat, 'mp3')
        assert.equal(options.network.retries, 3)
        assert.equal(options.extras.embedMetadata, true)
        assert.equal(options.ignoreConfig, true)
    })

    test('an unreadable group falls back to defaults', () => {
        const options = decodeOptions({ format: { mode: 'hologram' }, output: { template: '%(id)s.%(ext)s' } })
        assert.deepEqual(options.format, defaultOptions().format)
        assert.equal(options.output.template, '%(id)s.%(ext)s')
    })

    test('round-trips', () => {
        const options = defaultOptions()
        options.format.maxHeight = 720
        options.subtitles.enabled = true
        options.extras.sponsorCategories = ['intro']
        assert.deepEqual(decodeOptions(JSON.parse(JSON.stringify(options))), options)
    })
})

describe('FriendlyError', () => {
    test('maps raw errors', () => {
        for (const [raw, kind] of [
            ["[youtube] abc: Sign in to confirm you're not a bot. Use --cookies-from-browser", 'signInRequired'],
            ["[youtube] abc: Private video. Sign in if you've been granted access", 'signInRequired'],
            ['Failed to decrypt with DPAPI', 'browserCookies'],
            ['could not find chrome cookies database', 'browserCookies'],
            ['Unsupported URL: https://example.com/', 'unsupportedLink'],
            ["'nope' is not a valid URL", 'invalidLink'],
            ['[vimeo] 1: Video unavailable', 'unavailable'],
            ['Requested format is not available. Use --list-formats', 'formatUnavailable'],
            ['unable to download video data: HTTP Error 403: Forbidden', 'blocked'],
            ['[youtube] abc: nsig extraction failed: Some formats may be missing', 'blocked'],
            ['Unable to download webpage: <urlopen error [Errno 8] nodename nor servname provided>', 'network'],
            ['Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>', 'network'],
            ['[Errno 28] No space left on device', 'diskFull'],
            ['[Errno 13] Permission denied: \'/Volumes/Backup/x.mp4\'', 'folderNotWritable'],
            ['[WinError 5] Access is denied: \'D:\\\\x.mp4\'', 'folderNotWritable'],
            ['[youtube] abc: This live event will begin in 3 hours', 'notStarted'],
        ]) {
            assert.equal(friendlyErrorFromRaw(raw).kind, kind, raw)
        }
    })

    test('every known problem offers a fix', () => {
        assert.equal(fixFor(friendlyError('signInRequired')), Fix.browserSignIn)
        assert.equal(fixFor(friendlyError('blocked')), Fix.updateAndRetry)
        assert.equal(fixFor(friendlyError('folderNotWritable')), Fix.chooseFolder)
        assert.equal(fixFor(friendlyError('network')), Fix.retry)
        assert.equal(fixFor(friendlyError('formatUnavailable')), Fix.useBestQuality)
        assert.equal(fixFor(friendlyError('notStarted')), Fix.waitForStart)
        assert.equal(fixFor(friendlyError('invalidLink')), null)
    })

    test('messages don’t point at menus or tools', () => {
        for (const kind of ['browserCookies', 'signInRequired', 'blocked', 'folderNotWritable', 'notStarted']) {
            const message = friendlyError(kind).message
            assert.ok(!message.includes('→'))
            assert.ok(!message.includes('yt-dlp'))
        }
    })

    test('unknown errors lose the extractor prefix', () => {
        const error = friendlyErrorFromRaw('[generic] xyz: Something odd happened')
        assert.equal(error.kind, 'other')
        assert.equal(error.message, 'Something odd happened')
    })
})

describe('MediaInfo', () => {
    const json = {
        _type: 'video',
        title: 'Clip',
        uploader: 'Someone',
        duration: 61,
        thumbnails: [{ url: 'https://img/1.jpg' }, { url: 'https://img/2.jpg' }],
        formats: [
            { format_id: '140', ext: 'm4a', vcodec: 'none', acodec: 'mp4a.40.2' },
            { format_id: 'sb0', ext: 'mhtml', vcodec: 'none', acodec: 'none' },
            { format_id: '137', ext: 'mp4', height: 1080, fps: 60, vcodec: 'avc1', acodec: 'none' },
            { format_id: '22', ext: 'mp4', height: 720, fps: 30, vcodec: 'avc1', acodec: 'mp4a' },
        ],
        subtitles: { de: [], live_chat: [] },
        automatic_captions: { en: [], fr: [], 'ja-orig': [], ja: [] },
    }

    test('reads the parts the window shows', () => {
        const info = parseMediaInfo(json, 'https://x', 'en-US')
        assert.equal(info.subtitleTracks[0].title, 'German')
        assert.equal(info.thumbnail, 'https://img/2.jpg')
        assert.deepEqual(
            info.formats.map((f) => f.id),
            ['22', '137', '140'],
        )
        assert.equal(info.formats[1].resolution, '1080p60')
        assert.deepEqual(info.availableHeights, [1080, 720])
        assert.deepEqual(
            info.subtitleTracks.map((t) => `${t.code}${t.isAutomatic ? '*' : ''}`),
            ['de', 'en*', 'ja*'],
        )
        assert.equal(defaultSubtitleCode(info, null, 'tr-TR'), 'en')
        assert.equal(defaultSubtitleCode(info, 'de', 'tr-TR'), 'de')
    })

    test('file name preview', () => {
        assert.equal(renderTemplate('%(title)s.%(ext)s', null, { ext: 'mp4' }), 'Example Video Title.mp4')
        assert.equal(
            renderTemplate('%(playlist_index)03d - %(upload_date>%Y-%m-%d)s.%(ext)s', null, { ext: 'mp3' }),
            '001 - 2024-01-31.mp3',
        )
        assert.equal(renderTemplate('%(title)s', { title: 'A: B/C?' }, { ext: 'mp4' }), 'A - B⧸C?')
        assert.equal(renderTemplate('%(title)s', { title: 'A: B/C?' }, { ext: 'mp4', platform: 'win32' }), 'A - B⧸C？')
        assert.equal(renderTemplate('%(title)s', { title: 'Çay ve kedi' }, { ext: 'mp4', restrict: true }), 'Cay_ve_kedi')
        assert.equal(renderTemplate('%(nope|fallback)s', null, { ext: 'mp4' }), 'fallback')
    })
})
