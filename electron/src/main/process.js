// Running helper processes: output as lines, or collected. A port of PurrloaderCore's ChildProcess.swift.

import { spawn } from 'node:child_process'
import path from 'node:path'

import { AppPaths } from './store.js'

const isWindows = process.platform === 'win32'

/** Accumulates pipe output and hands back complete lines, treating \r as a line break too. */
export class LineBuffer {
    constructor() {
        this.pending = Buffer.alloc(0)
    }

    append(chunk) {
        const data = this.pending.length ? Buffer.concat([this.pending, chunk]) : chunk
        const lines = []
        let start = 0
        for (let i = 0; i < data.length; i++) {
            const byte = data[i]
            if (byte === 0x0a || byte === 0x0d) {
                if (i > start) lines.push(data.toString('utf8', start, i))
                start = i + 1
            }
        }
        this.pending = data.subarray(start)
        return lines
    }

    flush() {
        const rest = this.pending.toString('utf8')
        this.pending = Buffer.alloc(0)
        return rest ? [rest] : []
    }
}

/** PATH with Purrloader's tools first, so yt-dlp finds ffprobe, Deno and aria2c. */
export function toolEnvironment(extraDirs = []) {
    const env = { ...process.env }
    const key = isWindows ? Object.keys(env).find((name) => name.toUpperCase() === 'PATH') ?? 'Path' : 'PATH'
    const system = isWindows
        ? []
        : ['/usr/local/bin', '/usr/bin', '/bin', '/usr/sbin', '/sbin', ...(process.platform === 'darwin' ? ['/opt/homebrew/bin'] : [])]
    env[key] = [...extraDirs, AppPaths.bin, env[key] ?? '', ...system].filter(Boolean).join(path.delimiter)
    env.PYTHONIOENCODING = 'utf-8'
    return env
}

/** A running helper. `onLine` gets each line of stdout and stderr; `exited` resolves with the exit code once the
 * process has exited and its output was delivered. */
export class ChildProcess {
    constructor(executable, args, { cwd, env, onLine } = {}) {
        this.child = spawn(executable, args, {
            cwd,
            env: env ?? toolEnvironment(),
            stdio: ['ignore', 'pipe', 'pipe'],
            // No console window flashing up on Windows for every yt-dlp run.
            windowsHide: true,
        })
        this.running = true
        const buffers = { out: new LineBuffer(), err: new LineBuffer() }
        let openPipes = 2
        let exitCode = null
        let finished = false
        let resolveExit
        this.exited = new Promise((resolve) => (resolveExit = resolve))

        const finish = () => {
            if (finished || openPipes > 0 || exitCode === null) return
            finished = true
            for (const line of buffers.out.flush()) onLine?.(line, false)
            for (const line of buffers.err.flush()) onLine?.(line, true)
            resolveExit(exitCode)
        }
        for (const [stream, buffer, isError] of [
            [this.child.stdout, buffers.out, false],
            [this.child.stderr, buffers.err, true],
        ]) {
            stream.on('data', (chunk) => {
                for (const line of buffer.append(chunk)) onLine?.(line, isError)
            })
            stream.on('close', () => {
                openPipes--
                finish()
            })
        }
        this.child.on('error', (error) => {
            this.running = false
            this.error = error
            exitCode ??= -1
            openPipes = 0
            finish()
        })
        this.child.on('exit', (code, signal) => {
            this.running = false
            exitCode = code ?? (signal ? 1 : 0)
            finish()
            // A grandchild (ffmpeg) can hold the pipes open after yt-dlp is stopped; don't wait on it forever.
            setTimeout(() => {
                openPipes = 0
                finish()
            }, 3000).unref()
        })
    }

    get pid() {
        return this.child.pid
    }

    /** Like Ctrl-C: yt-dlp stops ffmpeg and leaves resumable .part files behind. Windows has no Ctrl-C for
     * windowless processes, so there the whole process tree stops at once; the .part files still resume. */
    interrupt() {
        if (!this.running) return
        if (isWindows) this.killTree()
        else this.child.kill('SIGINT')
    }

    terminate() {
        if (!this.running) return
        if (isWindows) this.killTree()
        else this.child.kill('SIGTERM')
    }

    killTree() {
        if (!this.child.pid) return
        spawn('taskkill', ['/pid', String(this.child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' }).on(
            'error',
            () => this.child.kill(),
        )
    }

    /** Runs a process to the end and collects its output. */
    static run(executable, args, { cwd, env, timeout = 0, onSpawn } = {}) {
        return new Promise((resolve) => {
            const stdout = []
            const stderr = []
            const child = spawn(executable, args, {
                cwd,
                env: env ?? toolEnvironment(),
                stdio: ['ignore', 'pipe', 'pipe'],
                windowsHide: true,
                timeout,
            })
            child.stdout.on('data', (chunk) => stdout.push(chunk))
            child.stderr.on('data', (chunk) => stderr.push(chunk))
            child.on('error', (error) =>
                resolve({ status: -1, stdout: Buffer.alloc(0), stderr: Buffer.from(String(error)), error, child }),
            )
            child.on('close', (code) =>
                resolve({ status: code ?? 1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr), child }),
            )
            onSpawn?.(child)
        })
    }
}

/** Finds an executable on PATH, as `which` does. */
export async function which(name) {
    const fs = await import('node:fs')
    const env = toolEnvironment()
    const pathValue = Object.entries(env).find(([key]) => key.toUpperCase() === 'PATH')?.[1] ?? ''
    const names = isWindows ? [`${name}.exe`, `${name}.cmd`, name] : [name]
    for (const dir of pathValue.split(path.delimiter).filter(Boolean)) {
        for (const candidate of names) {
            const full = path.join(dir, candidate)
            try {
                fs.accessSync(full, isWindows ? fs.constants.F_OK : fs.constants.X_OK)
                if (fs.statSync(full).isFile()) return full
            } catch {}
        }
    }
    return null
}
