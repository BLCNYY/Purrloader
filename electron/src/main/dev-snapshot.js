// Development only: runs a scenario script against the running app and saves pictures of its windows, so the
// look can be checked against the Mac app without driving the screen. Never loaded in a packaged build.
//
//   PURRLOADER_SNAPSHOT=/tmp/shots PURRLOADER_SCENARIO=./scenario.mjs npm start

import { BrowserWindow, app } from 'electron'
import fs from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

import { Downloads } from './downloads.js'
import { Model } from './model.js'
import { Hub } from './store.js'
import { Tools } from './tools.js'
import { Windows } from './windows.js'

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

async function loaded(window) {
    if (window.webContents.isLoading()) await new Promise((resolve) => window.webContents.once('did-finish-load', resolve))
    await wait(600)
}

export async function runSnapshots(dir, scenario) {
    fs.mkdirSync(dir, { recursive: true })
    const log = (...parts) => {
        console.log('[snapshot]', ...parts)
        fs.appendFileSync(path.join(dir, 'log.txt'), parts.join(' ') + '\n')
    }
    log('starting')
    const ctx = {
        Windows,
        Model,
        Downloads,
        Tools,
        Hub,
        wait,
        log,
        async capture(window, name) {
            await loaded(window)
            Hub.flush()
            await wait(250)
            const image = await window.webContents.capturePage()
            fs.writeFileSync(path.join(dir, `${name}.png`), image.toPNG())
            log('captured', name, JSON.stringify(image.getSize()))
        },
        /** Runs code in a window and returns its result. */
        exec(window, code) {
            return window.webContents.executeJavaScript(code, true)
        },
        /** Clicks the first element whose text or aria-label matches. */
        click(window, text, selector = 'button, [role=option], tr') {
            return window.webContents.executeJavaScript(
                `(() => {
                    const match = [...document.querySelectorAll(${JSON.stringify(selector)})].find((el) =>
                        el.textContent.trim() === ${JSON.stringify(text)} || el.getAttribute('aria-label') === ${JSON.stringify(text)})
                    if (!match) return false
                    match.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
                    match.click()
                    return true
                })()`,
                true,
            )
        },
    }
    try {
        const main = Windows.main
        main.webContents.on('console-message', (event) => log('console:', event.message ?? ''))
        await loaded(main)
        log('main window loaded')
        if (scenario) {
            const module = await import(pathToFileURL(path.resolve(scenario)).href)
            await module.default(ctx)
        } else {
            await ctx.capture(main, 'main')
        }
    } catch (error) {
        log('scenario failed:', error?.stack ?? String(error))
    }
    log('done')
    if (!process.env.PURRLOADER_KEEP_OPEN) {
        app.isQuitting = true
        for (const window of BrowserWindow.getAllWindows()) window.destroy()
        app.exit(0)
    }
}
