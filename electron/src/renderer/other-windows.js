// The History window and the tray panel. Ports of Features/Queue/History.swift and App/MenuBar.swift.

import { CircleButton, PrimaryButton, SlimProgress, Thumbnail, openMenu } from './controls.js'
import { Icon } from './icons.js'
import { cx, html, invoke, isMac, isShortcut, useAppState, useEffect, useLayoutEffect, useRef, useState } from './lib.js'

// MARK: - History

export function HistoryWindow() {
    const { history, app } = useAppState()
    const [search, setSearch] = useState('')
    const [selection, setSelection] = useState(new Set())
    const [anchor, setAnchor] = useState(null)
    const query = search.trim().toLowerCase()
    const filtered = query
        ? history.filter((entry) => entry.title.toLowerCase().includes(query) || entry.url.toLowerCase().includes(query))
        : history
    const date = new Intl.DateTimeFormat(undefined, { day: 'numeric', month: 'short', year: 'numeric', hour: 'numeric', minute: '2-digit' })

    useEffect(() => {
        document.title = 'History'
        const key = (event) => {
            if (isShortcut(event) && event.key.toLowerCase() === 'a' && document.activeElement?.tagName !== 'INPUT') {
                event.preventDefault()
                setSelection(new Set(filtered.map((entry) => entry.id)))
            }
        }
        document.addEventListener('keydown', key)
        return () => document.removeEventListener('keydown', key)
    })

    const select = (event, entry, index) => {
        if (event.shiftKey && anchor != null) {
            const [from, to] = [Math.min(anchor, index), Math.max(anchor, index)]
            setSelection(new Set(filtered.slice(from, to + 1).map((e) => e.id)))
        } else if (isShortcut(event)) {
            const next = new Set(selection)
            next.has(entry.id) ? next.delete(entry.id) : next.add(entry.id)
            setSelection(next)
            setAnchor(index)
        } else {
            setSelection(new Set([entry.id]))
            setAnchor(index)
        }
    }

    const clearHistory = async () => {
        const choice = await invoke('confirm', {
            message: 'Clear all download history?',
            detail: 'Downloaded files are not affected.',
            buttons: ['Clear History', 'Cancel'],
        })
        if (choice === 0) invoke('history.clear')
    }

    const contextMenu = (event, entry) => {
        event.preventDefault()
        const ids = selection.has(entry.id) ? [...selection] : [entry.id]
        if (!selection.has(entry.id)) setSelection(new Set([entry.id]))
        openMenu({
            anchor: { x: event.clientX, y: event.clientY },
            items: [
                { label: 'Download Again', action: () => invoke('history.redownload', ids) },
                { label: `Show in ${app.fileManager}`, action: () => invoke('history.reveal', ids[0]) },
                { label: 'Copy Link', action: () => invoke('history.copyLinks', ids) },
                { separator: true },
                { label: 'Remove from History', action: () => invoke('history.delete', ids) },
            ],
        })
    }

    return html`<div class=${cx('window', isMac && 'mac')}>
        <div class="toolbar history-toolbar wco-right">
            <span class="title">History</span>
            <button class="toolbar-button" title="Download Again" aria-label="Download Again" disabled=${!selection.size} onClick=${() => invoke('history.redownload', [...selection])}>
                <${Icon} name="arrowDownCircle" size=${17} weight="regular" />
            </button>
            <button class="toolbar-button" title="Clear History" aria-label="Clear History" disabled=${!history.length} onClick=${clearHistory}>
                <${Icon} name="trash" size=${17} weight="regular" />
            </button>
            <label class="search-field no-drag">
                <${Icon} name="magnifyingglass" size=${13} weight="medium" />
                <input type="search" placeholder="Search history" value=${search} spellcheck=${false} onInput=${(event) => setSearch(event.target.value)} />
            </label>
            <span style=${{ width: '12px' }} />
        </div>
        <div class="history-table">
            <table role="grid" aria-multiselectable="true">
                <colgroup><col /><col style=${{ width: '170px' }} /><col style=${{ width: '190px' }} /></colgroup>
                <thead><tr><th>Title</th><th>Date</th><th>Folder</th></tr></thead>
                <tbody>
                    ${filtered.map(
                        (entry, index) => html`<tr
                            key=${entry.id}
                            class=${cx(selection.has(entry.id) && 'selected')}
                            aria-selected=${selection.has(entry.id)}
                            onMouseDown=${(event) => event.button === 0 && select(event, entry, index)}
                            onDblClick=${() => invoke('history.reveal', entry.id)}
                            onContextMenu=${(event) => contextMenu(event, entry)}
                        >
                            <td>
                                <div class="row" style=${{ gap: '8px' }}>
                                    <${Thumbnail} url=${entry.thumbnail} width=${48} />
                                    <span class="ellipsis">${entry.title}</span>
                                </div>
                            </td>
                            <td class="secondary">${date.format(new Date(entry.date))}</td>
                            <td class="secondary" title=${entry.folder}>${shortFolder(entry.folder, app)}</td>
                        </tr>`,
                    )}
                </tbody>
            </table>
            ${!history.length &&
            html`<div class="unavailable">
                <${Icon} name="clock" size=${34} weight="regular" color="var(--tertiary)" />
                <h2>No History</h2>
                <div class="secondary">Finished downloads show up here.</div>
            </div>`}
            ${history.length > 0 &&
            !filtered.length &&
            html`<div class="unavailable">
                <${Icon} name="magnifyingglass" size=${34} weight="regular" color="var(--tertiary)" />
                <h2>No Results for “${search.trim()}”</h2>
                <div class="secondary">Check the spelling or try a new search.</div>
            </div>`}
        </div>
    </div>`
}

function shortFolder(folder, app) {
    return app.platform === 'win32' ? folder : folder.replace(/^\/(home|Users)\/[^/]+/, '~')
}

// MARK: - Tray panel

/** What the Mac app shows from the menu bar: what's downloading, and a way back to the window. */
export function TrayWindow() {
    const { jobs } = useAppState()
    const panel = useRef(null)
    const active = jobs.activeCount
    const queued = jobs.queuedCount
    const summary = active === 0 && queued === 0 ? 'Idle' : queued > 0 ? `${active} active · ${queued} waiting` : `${active} active`
    const visible = jobs.jobs.filter((job) => !job.isDone).slice(0, 6)

    useLayoutEffect(() => {
        if (panel.current) invoke('window.fitTray', panel.current.offsetHeight)
    })

    return html`<div class="tray-panel" ref=${panel}>
        <div class="row">
            <span class="spacer" style=${{ fontSize: '14px', fontWeight: 600 }}>Purrloader</span>
            <span style=${{ fontSize: '11px', color: 'var(--secondary)' }}>${summary}</span>
        </div>
        ${visible.length
            ? visible.map(
                  (job) => html`<div key=${job.id} class="row" style=${{ gap: '10px' }}>
                      <${Thumbnail} url=${job.thumbnail} width=${44} />
                      <div style=${{ display: 'flex', flexDirection: 'column', gap: '5px', minWidth: 0, flex: 1 }}>
                          <div class="ellipsis" style=${{ fontSize: '12px', fontWeight: 600 }}>${job.title}</div>
                          ${job.isActive && html`<${SlimProgress} fraction=${job.fraction} />`}
                          <div class="ellipsis" style=${{ fontSize: '10px', color: 'var(--secondary)' }}>${job.statusText}</div>
                      </div>
                  </div>`,
              )
            : html`<div style=${{ minHeight: '44px', display: 'grid', placeItems: 'center', fontSize: '12px', color: 'var(--tertiary)' }}>Nothing downloading</div>`}
        <div class="row" style=${{ paddingTop: '2px' }}>
            <${PrimaryButton} onClick=${() => invoke('window.showMain')}>Open Purrloader<//>
            <span class="spacer" />
            ${active > 0 || queued > 0
                ? html`<${CircleButton} icon="pauseFill" size=${30} label="Pause all" onClick=${() => invoke('jobs.pauseAll')} />`
                : jobs.hasPaused && html`<${CircleButton} icon="playFill" size=${30} label="Resume all" onClick=${() => invoke('jobs.resumeAll')} />`}
            <${CircleButton} icon="power" size=${30} label="Quit Purrloader" onClick=${() => invoke('app.quit')} />
        </div>
    </div>`
}
