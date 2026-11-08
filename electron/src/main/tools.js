// Installs, verifies and updates the helper tools, and reports their versions.
// A port of PurrloaderCore's ToolManager.swift, with Windows and Linux builds of each tool.

import { net } from 'electron'
import { createHash, randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { pipeline } from 'node:stream/promises'
import yauzl from 'yauzl'

import { checksumFor } from '../core/support.js'
import { ChildProcess } from './process.js'
import { AppPaths, Hub, Settings } from './store.js'

export const TOOLS = ['yt-dlp', 'ffmpeg', 'ffprobe', 'deno']

const platform = process.platform
const arm = process.arch === 'arm64'
const exe = (name) => (platform === 'win32' ? `${name}.exe` : name)

/** What gets installed for each tool on this computer. FFmpeg's archive also holds ffprobe, so they share a
 * package everywhere but on the Mac, where they come as separate zips. */
const PACKAGES = {
    'yt-dlp': {
        // The folder ("onedir") build starts in a fraction of a second, where the single-file build unpacks itself
        // for seconds on every run.
        executables: [
            platform === 'win32'
                ? arm
                    ? 'yt-dlp_arm64.exe'
                    : 'yt-dlp.exe'
                : platform === 'linux'
                  ? arm
                      ? 'yt-dlp_linux_aarch64'
                      : 'yt-dlp_linux'
                  : 'yt-dlp_macos',
        ],
        async sources(channel) {
            const repository = channel === 'nightly' ? 'yt-dlp/yt-dlp-nightly-builds' : 'yt-dlp/yt-dlp'
            const tag = await latestTag(repository)
            const base = `https://github.com/${repository}/releases/download/${tag}/`
            const asset =
                platform === 'win32'
                    ? arm
                        ? 'yt-dlp_win_arm64.zip'
                        : 'yt-dlp_win.zip'
                    : platform === 'linux'
                      ? arm
                          ? 'yt-dlp_linux_aarch64.zip'
                          : 'yt-dlp_linux.zip'
                      : 'yt-dlp_macos.zip'
            // yt-dlp publishes one SHA2-256SUMS file per release.
            return [{ archive: base + asset, checksums: base + 'SHA2-256SUMS' }]
        },
    },
    ffmpeg: {
        executables: [exe('ffmpeg'), exe('ffprobe')],
        async sources() {
            if (platform === 'darwin') {
                // martin-riedl.de redirects "latest" to a versioned path that has a .sha256 next to the zip.
                const arch = arm ? 'arm64' : 'amd64'
                return Promise.all(
                    ['ffmpeg', 'ffprobe'].map(async (tool) => {
                        const archive = await finalURL(
                            `https://ffmpeg.martin-riedl.de/redirect/latest/macos/${arch}/release/${tool}.zip`,
                        )
                        return { archive, checksums: archive + '.sha256' }
                    }),
                )
            }
            // yt-dlp's own FFmpeg builds, patched for the problems yt-dlp runs into. On Windows the "shared" build
            // keeps ffmpeg and ffprobe from each carrying a full copy of the libraries: half the download.
            const base = 'https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/'
            const asset =
                platform === 'win32'
                    ? `ffmpeg-master-latest-${arm ? 'winarm64' : 'win64'}-gpl-shared.zip`
                    : `ffmpeg-master-latest-${arm ? 'linuxarm64' : 'linux64'}-gpl.tar.xz`
            return [{ archive: base + asset, checksums: base + 'checksums.sha256' }]
        },
    },
    deno: {
        executables: [exe('deno')],
        async sources() {
            const triple = {
                win32: `${arm ? 'aarch64' : 'x86_64'}-pc-windows-msvc`,
                linux: `${arm ? 'aarch64' : 'x86_64'}-unknown-linux-gnu`,
                darwin: `${arm ? 'aarch64' : 'x86_64'}-apple-darwin`,
            }[platform]
            const tag = await latestTag('denoland/deno')
            const archive = `https://github.com/denoland/deno/releases/download/${tag}/deno-${triple}.zip`
            return [{ archive, checksums: archive + '.sha256sum' }]
        },
    },
}

const PACKAGE_OF = { 'yt-dlp': 'yt-dlp', ffmpeg: 'ffmpeg', ffprobe: 'ffmpeg', deno: 'deno' }

/** Each package version gets its own folder under bin/<package>/, and bin/<package>/current names the newest.
 * Downloads launch the resolved path, so an update never swaps files out from under a running yt-dlp or ffmpeg
 * (Windows can't replace a running .exe at all). Old versions are removed at the next launch. */
function packageDir(name) {
    return path.join(AppPaths.bin, name)
}

function currentBuild(name) {
    try {
        return fs.readFileSync(path.join(packageDir(name), 'current'), 'utf8').trim() || null
    } catch {
        return null
    }
}

/** The executable for a tool, or null if it isn't installed. */
export function toolPath(tool) {
    const name = PACKAGE_OF[tool]
    const build = currentBuild(name)
    if (!build) return null
    const executable = tool === 'ffprobe' ? exe('ffprobe') : PACKAGES[name].executables[0]
    return path.join(packageDir(name), build, executable)
}

function isInstalled(tool) {
    const file = toolPath(tool)
    if (!file) return false
    try {
        fs.accessSync(file, platform === 'win32' ? fs.constants.F_OK : fs.constants.X_OK)
        return true
    } catch {
        return false
    }
}

export class ToolError extends Error {
    constructor(kind, tool, detail) {
        const messages = {
            http: `The server answered with HTTP ${detail}.`,
            unexpectedResponse: 'The server sent something unexpected.',
            missingChecksum: `No published checksum was found for ${tool}.`,
            checksumMismatch: `The downloaded ${tool} didn't match its published checksum, so it was deleted.`,
            unpack: `Couldn't unpack ${tool}.`,
            missingExecutable: `${tool} wasn't where expected in its download.`,
        }
        super(messages[kind] ?? 'Something went wrong.')
        this.kind = kind
    }
}

// MARK: - Network

/** Where a URL ends up after its redirects. net.fetch doesn't report that, so this follows them one by one and
 * stops reading once the headers of the last answer arrive. GET rather than HEAD: martin-riedl.de answers HEAD
 * with 404. */
function finalURL(url) {
    return new Promise((resolve, reject) => {
        let current = url
        const request = net.request({ url, method: 'GET', redirect: 'manual' })
        request.on('redirect', (_status, _method, redirectURL) => {
            current = redirectURL
            request.followRedirect()
        })
        request.on('response', (response) => {
            const status = response.statusCode
            request.abort()
            if (status === 200) resolve(current)
            else reject(new ToolError('unexpectedResponse'))
        })
        request.on('error', reject)
        request.end()
    })
}

/** The newest release tag, read from where github.com/…/releases/latest redirects. Unlike the API, this has no
 * rate limit. */
export async function latestTag(repository) {
    const url = new URL(await finalURL(`https://github.com/${repository}/releases/latest`))
    const parts = url.pathname.split('/').filter(Boolean)
    if (parts.at(-2) !== 'tag') throw new ToolError('unexpectedResponse')
    return decodeURIComponent(parts.at(-1))
}

/** Downloads to a file, reporting progress, and returns the file's SHA-256. */
async function download(url, destination, onProgress) {
    const response = await net.fetch(url, { cache: 'no-store' })
    if (response.status !== 200 || !response.body) throw new ToolError('http', null, response.status)
    const expected = Number(response.headers.get('content-length') || 0)
    const hash = createHash('sha256')
    let written = 0
    const output = fs.createWriteStream(destination)
    await pipeline(
        response.body,
        async function* (source) {
            for await (const chunk of source) {
                hash.update(chunk)
                written += chunk.length
                onProgress(written, expected)
                yield chunk
            }
        },
        output,
    )
    return hash.digest('hex')
}

async function fetchText(url) {
    const response = await net.fetch(url, { cache: 'no-store' })
    if (response.status !== 200) throw new ToolError('http', null, response.status)
    return response.text()
}

// MARK: - Unpacking

function unzip(file, destination) {
    return new Promise((resolve, reject) => {
        yauzl.open(file, { lazyEntries: true }, (error, zip) => {
            if (error) return reject(error)
            const root = path.resolve(destination)
            zip.on('error', reject)
            zip.on('end', resolve)
            zip.on('entry', (entry) => {
                const target = path.resolve(root, entry.fileName)
                // Never write outside the folder, whatever names the archive holds.
                if (target !== root && !target.startsWith(root + path.sep)) return reject(new Error('Unsafe path in archive'))
                if (entry.fileName.endsWith('/')) {
                    fs.mkdirSync(target, { recursive: true })
                    return zip.readEntry()
                }
                fs.mkdirSync(path.dirname(target), { recursive: true })
                zip.openReadStream(entry, (streamError, stream) => {
                    if (streamError) return reject(streamError)
                    const mode = (entry.externalFileAttributes >>> 16) & 0o777
                    pipeline(stream, fs.createWriteStream(target, mode ? { mode } : undefined))
                        .then(() => zip.readEntry())
                        .catch(reject)
                })
            })
            zip.readEntry()
        })
    })
}

async function unpack(archive, destination) {
    fs.mkdirSync(destination, { recursive: true })
    if (archive.endsWith('.zip')) return unzip(archive, destination)
    // FFmpeg for Linux comes as .tar.xz; every Linux has tar and xz.
    const result = await ChildProcess.run('tar', ['-xJf', archive, '-C', destination])
    if (result.status !== 0) throw new Error(result.stderr.toString())
}

function findFile(root, name) {
    for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
        const full = path.join(root, entry.name)
        if (entry.isFile() && entry.name === name) return full
        if (entry.isDirectory()) {
            const found = findFile(full, name)
            if (found) return found
        }
    }
    return null
}

// MARK: - Manager

/** Download servers hiccup now and then, so try a few times. A checksum mismatch is final. */
async function retrying(attempt) {
    for (const delay of [1000, 3000]) {
        try {
            return await attempt()
        } catch (error) {
            if (error instanceof ToolError && error.kind === 'checksumMismatch') throw error
        }
        await new Promise((resolve) => setTimeout(resolve, delay))
    }
    return attempt()
}

async function installPackage(name, channel, onProgress) {
    const pkg = PACKAGES[name]
    const sources = await pkg.sources(channel)
    const work = path.join(AppPaths.temp, `install-${name}-${randomUUID()}`)
    fs.mkdirSync(work, { recursive: true })
    try {
        const sizes = sources.map(() => ({ written: 0, expected: 0 }))
        const report = () =>
            onProgress(
                sizes.reduce((sum, s) => sum + s.written, 0),
                sizes.reduce((sum, s) => sum + s.expected, 0),
            )
        await Promise.all(
            sources.map(async (source, index) => {
                const fileName = decodeURIComponent(new URL(source.archive).pathname.split('/').at(-1))
                const archive = path.join(work, fileName)
                const sumsText = fetchText(source.checksums)
                const actual = await download(source.archive, archive, (written, expected) => {
                    sizes[index] = { written, expected }
                    report()
                })
                let expected
                try {
                    expected = checksumFor(fileName, await sumsText)
                } catch {
                    expected = null
                }
                if (!expected) throw new ToolError('missingChecksum', name)
                // Checked before anything in the archive is unpacked or run.
                if (actual !== expected) {
                    fs.rmSync(archive, { force: true })
                    throw new ToolError('checksumMismatch', name)
                }
                try {
                    await unpack(archive, path.join(work, `unpacked-${index}`))
                } catch {
                    throw new ToolError('unpack', name)
                }
                fs.rmSync(archive, { force: true })
            }),
        )

        const build = randomUUID()
        const buildDir = path.join(packageDir(name), build)
        fs.mkdirSync(packageDir(name), { recursive: true })
        for (const [index, executable] of pkg.executables.entries()) {
            if (fs.existsSync(path.join(buildDir, executable))) continue
            const found = findFile(work, executable)
            if (!found) throw new ToolError('missingExecutable', name)
            if (index === 0) fs.renameSync(path.dirname(found), buildDir)
            else fs.renameSync(found, path.join(buildDir, executable))
        }
        for (const executable of pkg.executables) fs.chmodSync(path.join(buildDir, executable), 0o755)
        // Point "current" at the new build in one step: write beside it, then rename over the old.
        const pointer = path.join(packageDir(name), 'current')
        fs.writeFileSync(`${pointer}.tmp`, build)
        fs.renameSync(`${pointer}.tmp`, pointer)
    } finally {
        fs.rmSync(work, { recursive: true, force: true })
    }
}

export function parseVersion(output, tool) {
    const first = String(output).split('\n')[0]?.trim() ?? ''
    if (tool === 'ffmpeg' || tool === 'ffprobe') {
        // "ffmpeg version 8.0-https://… Copyright …" -> "8.0"; git builds read
        // "ffmpeg version N-121595-gb2b8c3f9a5-20251103 …" -> "N-121595 (2025-11-03)".
        const token = first.split(' ')[2]
        if (!token) return first
        const git = token.match(/^N-(\d+)-g[0-9a-f]+(?:-(\d{4})(\d{2})(\d{2}))?/)
        if (git) return git[2] ? `N-${git[1]} (${git[2]}-${git[3]}-${git[4]})` : `N-${git[1]}`
        return token.replace(/^n(?=\d)/, '').split('-')[0]
    }
    if (tool === 'deno') return first.split(' ')[1] ?? first // "deno 2.5.0 (stable, …)" -> "2.5.0"
    return first
}

export const Tools = {
    versions: {},
    /** Tools being downloaded right now, with their progress. */
    transfers: {},
    /** Shown when the helper tools are missing and couldn't be installed. */
    setupError: null,
    /** Shown when a silent yt-dlp update failed; cleared on success or dismissal. */
    updateError: null,
    isUpdating: false,
    updateLog: '',
    updatePromise: null,

    get channel() {
        return Settings.get('ytdlpChannel') === 'nightly' ? 'nightly' : 'stable'
    },

    get isReady() {
        return TOOLS.every((tool) => this.versions[tool] != null)
    },

    get isInstalling() {
        return Object.keys(this.transfers).length > 0
    },

    path: toolPath,

    changed() {
        Hub.changed('tools', 'model')
    },

    /** Combined progress of the tools being installed, for the welcome sheet. */
    get installProgress() {
        const transfers = Object.values(this.transfers)
        if (!transfers.length) return null
        return transfers.reduce(
            (total, t) => ({ written: total.written + t.written, expected: total.expected + Math.max(t.expected, 0) }),
            { written: 0, expected: 0 },
        )
    },

    /** Installs whatever is missing, reads versions, then quietly updates yt-dlp. */
    async bootstrap() {
        this.setupError = null
        this.changed()
        this.cleanUp()
        const missing = TOOLS.filter((tool) => !isInstalled(tool))
        if (missing.length) {
            try {
                await this.install(missing)
            } catch (error) {
                console.error('Installing tools failed:', error)
                this.setupError = error.message || String(error)
            }
        }
        await this.refreshVersions()
        if (!missing.length && Settings.get('autoUpdateYtdlp')) await this.updateYtdlp({ silently: true })
    },

    /** Downloads tools in parallel. Each archive is checked against its publisher's SHA-256 before anything in it
     * is unpacked or made executable. */
    async install(tools) {
        const packages = [...new Set(tools.map((tool) => PACKAGE_OF[tool]))]
        for (const tool of tools) this.transfers[tool] = { written: 0, expected: 0 }
        this.changed()
        const channel = this.channel
        try {
            await Promise.all(
                packages.map((name) =>
                    retrying(() =>
                        installPackage(name, channel, (written, expected) => {
                            // One transfer per package, so ffmpeg's archive isn't counted twice.
                            const key = name === 'ffmpeg' ? (tools.includes('ffmpeg') ? 'ffmpeg' : 'ffprobe') : name
                            if (this.transfers[key]) {
                                this.transfers[key] = { written, expected }
                                Hub.changed('tools')
                            }
                        }),
                    ),
                ),
            )
        } finally {
            for (const tool of tools) delete this.transfers[tool]
            this.changed()
        }
    },

    /** Removes leftovers from earlier runs: unpack folders and old tool versions. Runs before any download starts,
     * so nothing is in use. */
    cleanUp() {
        for (const name of safeReaddir(AppPaths.temp)) {
            if (name.startsWith('install-') || name.startsWith('old-')) {
                fs.rmSync(path.join(AppPaths.temp, name), { recursive: true, force: true })
            }
        }
        for (const name of Object.keys(PACKAGES)) {
            const current = currentBuild(name)
            for (const entry of safeReaddir(packageDir(name))) {
                if (entry === 'current' || entry === current) continue
                try {
                    fs.rmSync(path.join(packageDir(name), entry), { recursive: true, force: true })
                } catch {}
            }
        }
    },

    async reinstallAll() {
        this.setupError = null
        try {
            await this.install(TOOLS)
        } catch (error) {
            this.setupError = error.message || String(error)
        }
        await this.refreshVersions()
    },

    async refreshVersions() {
        // All version checks at once; a freshly installed binary's first launch can be slow while antivirus
        // software scans it.
        const results = await Promise.all(
            TOOLS.map(async (tool) => {
                if (!isInstalled(tool)) return [tool, null]
                const args = tool === 'ffmpeg' || tool === 'ffprobe' ? ['-version'] : ['--version']
                const result = await ChildProcess.run(toolPath(tool), args, { timeout: 120_000 })
                return [tool, result.status === 0 ? parseVersion(result.stdout.toString('utf8'), tool) : null]
            }),
        )
        this.versions = Object.fromEntries(results)
        this.changed()
    },

    /** Installs the newest yt-dlp for the current channel if it differs from the installed one. Several callers at
     * once share one update. A silent update only reports failures. */
    async updateYtdlp({ silently = false } = {}) {
        if (this.updatePromise) return this.updatePromise
        this.updatePromise = this.performUpdate(silently).finally(() => (this.updatePromise = null))
        return this.updatePromise
    },

    async performUpdate(silently) {
        this.isUpdating = true
        this.changed()
        const before = this.versions['yt-dlp']
        const channel = this.channel
        try {
            const repository = channel === 'nightly' ? 'yt-dlp/yt-dlp-nightly-builds' : 'yt-dlp/yt-dlp'
            const latest = await latestTag(repository)
            if (latest !== before) {
                await this.install(['yt-dlp'])
                await this.refreshVersions()
            }
            this.updateError = null
            const now = this.versions['yt-dlp']
            if (now && !silently) {
                const title = channel === 'nightly' ? 'Nightly' : 'Stable'
                this.updateLog = now === before ? `yt-dlp ${now} is the newest version.` : `Updated yt-dlp to ${now} (${title}).`
            }
        } catch (error) {
            console.error('Updating yt-dlp failed:', error)
            this.updateError = error.message || String(error)
            this.updateLog = `Couldn't update yt-dlp: ${this.updateError}`
        } finally {
            this.isUpdating = false
            this.changed()
        }
    },

    setChannel(channel) {
        Settings.set('ytdlpChannel', channel === 'nightly' ? 'nightly' : 'stable')
        this.updateYtdlp()
    },

    state() {
        return {
            versions: this.versions,
            transfers: this.transfers,
            setupError: this.setupError,
            updateError: this.updateError,
            isUpdating: this.isUpdating,
            isInstalling: this.isInstalling,
            isReady: this.isReady,
            updateLog: this.updateLog,
            installProgress: this.installProgress,
            channel: this.channel,
            binFolder: AppPaths.bin,
        }
    },
}

function safeReaddir(dir) {
    try {
        return fs.readdirSync(dir)
    } catch {
        return []
    }
}

Hub.provide('tools', () => Tools.state())
