// Picks the window to draw from ?view= and starts it once the first state has arrived.

import { MenuHost } from './controls.js'
import { html, loadState, render } from './lib.js'
import { MainWindow } from './main-window.js'
import { HistoryWindow, TrayWindow } from './other-windows.js'
import { SettingsWindow } from './settings-window.js'

const VIEWS = { main: MainWindow, settings: SettingsWindow, history: HistoryWindow, tray: TrayWindow }

const view = new URLSearchParams(location.search).get('view') ?? 'main'
const View = VIEWS[view] ?? MainWindow
document.body.classList.add(view, window.purrloader.platform)

// Dropping a file anywhere else would open it in place of the window.
for (const type of ['dragover', 'drop']) document.addEventListener(type, (event) => event.preventDefault())

await loadState()
render(html`<${View} /><${MenuHost} />`, document.getElementById('root'))
