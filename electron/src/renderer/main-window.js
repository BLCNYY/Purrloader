// The main window: link field, info card, playlist checklist, the More popover, the queue, the details sheet
// and the first-launch welcome. Ports of Features/Main, Features/Queue and Features/Onboarding.

import {
    BezelButton,
    ChipButton,
    ChipMenu,
    CircleButton,
    MiddleText,
    PillSegment,
    Popover,
    PopUpButton,
    PrimaryButton,
    SecondaryButton,
    Sheet,
    SlimProgress,
    Spinner,
    Switch,
    Tag,
    TextInput,
    Thumbnail,
    openMenu,
} from './controls.js'
import { Icon } from './icons.js'
import {
    cx,
    html,
    invoke,
    isMac,
    isShortcut,
    shortcutLabel,
    useAppState,
    useCallback,
    useEffect,
    useRef,
    useState,
} from './lib.js'
import { AudioFormat, CookieBrowser } from '../core/options.js'
import { FIX_TITLES, fixFor } from '../core/errors.js'
import { Format } from '../core/support.js'

const AUDIO_CHOICES = ['mp3', 'm4a', 'best']

export function MainWindow() {
    const state = useAppState()
    const { model, settings, jobs } = state
    const [dropTargeted, setDropTargeted] = useState(false)
    const [detailsJob, setDetailsJob] = useState(null)

    useShortcuts(model)

    const onDrop = async (event) => {
        event.preventDefault()
        setDropTargeted(false)
        const transfer = event.dataTransfer
        const files = [...transfer.files].map((file) => window.purrloader.pathForFile(file)).filter(Boolean)
        const strings = [transfer.getData('text/uri-list'), transfer.getData('text/plain')].filter(Boolean)
        invoke('model.acceptDropped', { files, strings })
    }

    return html`<div
        class=${cx('window', isMac && 'mac')}
        onDragOver=${(event) => {
            event.preventDefault()
            setDropTargeted(true)
        }}
        onDragLeave=${(event) => !event.relatedTarget && setDropTargeted(false)}
        onDrop=${onDrop}
    >
        <div class="titlebar"><${AppMenuButton} /></div>
        <${NoticeBanner} state=${state} />
        <div class="main-top">
            <${UrlField} model=${model} />
            <${LinkStatus} state=${state} />
            <${Controls} state=${state} />
        </div>
        <div class="rule" />
        <${QueueView} jobs=${jobs} app=${state.app} showDetails=${setDetailsJob} />
        ${dropTargeted &&
        html`<svg class="drop-outline" width="100%" height="100%" aria-hidden="true">
            <rect x="1" y="1" rx="16" style=${{ width: 'calc(100% - 2px)', height: 'calc(100% - 2px)' }} fill="none" stroke="rgba(255,255,255,0.6)" stroke-width="2" stroke-dasharray="8 6" />
        </svg>`}
        ${detailsJob && html`<${JobDetails} id=${detailsJob} onClose=${() => setDetailsJob(null)} />`}
        ${!settings.didShowWelcome && html`<${Welcome} tools=${state.tools} app=${state.app} />`}
    </div>`
}

// MARK: - Menu and shortcuts

function appMenuItems() {
    const { model } = window.__state ?? {}
    return [
        { label: 'Paste Link and Get Info', shortcut: shortcutLabel('Ctrl+Shift+V'), action: () => invoke('model.pasteAndFetch') },
        { label: 'Download', shortcut: shortcutLabel('Ctrl+Enter'), disabled: model && !model.canDownload, action: () => invoke('model.download') },
        { separator: true },
        { label: 'Advanced Options…', shortcut: shortcutLabel('Ctrl+Alt+O'), action: () => invoke('window.openSettings', 'advanced') },
        { label: 'Pause All', shortcut: shortcutLabel('Ctrl+Alt+P'), action: () => invoke('jobs.pauseAll') },
        { label: 'Resume All', shortcut: shortcutLabel('Ctrl+Alt+R'), action: () => invoke('jobs.resumeAll') },
        { label: 'Clear Recent', shortcut: shortcutLabel('Ctrl+K'), action: () => invoke('jobs.clearRecent') },
        { label: 'Update Helper Tools', shortcut: shortcutLabel('Ctrl+Shift+U'), action: () => invoke('tools.update') },
        { separator: true },
        { label: 'History', shortcut: shortcutLabel('Ctrl+Y'), action: () => invoke('window.openHistory') },
        { label: 'Settings…', shortcut: shortcutLabel('Ctrl+,'), action: () => invoke('window.openSettings', 'general') },
        { label: 'About Purrloader', action: () => invoke('window.openSettings', 'about') },
        { separator: true },
        { label: 'Quit Purrloader', shortcut: shortcutLabel('Ctrl+Q'), action: () => invoke('app.quit') },
    ]
}

/** Windows and Linux have no menu bar for Purrloader's window; this button holds the Mac's menu commands. */
function AppMenuButton() {
    const ref = useRef(null)
    const [open, setOpen] = useState(false)
    return html`<button
        ref=${ref}
        class=${cx('app-menu-button', open && 'open')}
        title="Menu"
        aria-label="Menu"
        aria-haspopup="menu"
        onClick=${() => {
            setOpen(true)
            openMenu({ items: appMenuItems(), anchor: ref.current, onClose: () => setOpen(false) })
        }}
    >
        <${Icon} name="lineHorizontal3" size=${14} weight="medium" />
    </button>`
}

function useShortcuts(model) {
    window.__state = { model }
    useEffect(() => {
        const handle = (event) => {
            if (!isShortcut(event) || document.querySelector('[data-menu-layer], .sheet-backdrop')) return
            const key = event.key.length === 1 ? event.key.toLowerCase() : event.key
            const run = (action, ...args) => {
                event.preventDefault()
                invoke(action, ...args)
            }
            if (key === 'Enter' && !event.shiftKey && !event.altKey) run('model.download')
            else if (key === 'v' && event.shiftKey) run('model.pasteAndFetch')
            else if (event.altKey && (event.code === 'KeyO' || key === 'o')) run('window.openSettings', 'advanced')
            else if (event.altKey && (event.code === 'KeyP' || key === 'p')) run('jobs.pauseAll')
            else if (event.altKey && (event.code === 'KeyR' || key === 'r')) run('jobs.resumeAll')
            else if (key === 'k' && !event.shiftKey && !event.altKey) run('jobs.clearRecent')
            else if (key === 'u' && event.shiftKey) run('tools.update')
            else if (key === 'y' && !event.shiftKey) run('window.openHistory')
            else if (key === ',') run('window.openSettings', 'general')
            else if (key === 'q' && !event.shiftKey) run('app.quit')
        }
        document.addEventListener('keydown', handle)
        return () => document.removeEventListener('keydown', handle)
    }, [])
}

// MARK: - Notice banner

/** One quiet line above the link field, only when something needs attention. */
function NoticeBanner({ state }) {
    const { tools, updater } = state
    const [busy, setBusy] = useState(false)
    let content = null
    if (!tools.isReady && !tools.setupError) {
        const progress = tools.installProgress
        content = html`<${Spinner} size="mini" />
            <span>Getting Purrloader ready…</span>
            <span class="spacer" />
            ${progress?.expected > 0 &&
            html`<span class="mono-digits">${Format.bytes(progress.written)} of ${Format.bytes(progress.expected)}</span>`}`
    } else if (tools.setupError) {
        content = html`<${Icon} name="exclamationTriangleFill" size=${13} color="var(--warning)" />
            <span class="lines-2" title=${tools.setupError}>Couldn't download Purrloader's helper tools. Check your internet connection.</span>
            <span class="spacer" />
            <${SecondaryButton} onClick=${() => invoke('tools.retrySetup')}>Retry<//>`
    } else if (tools.updateError) {
        content = html`<${Icon} name="exclamationTriangleFill" size=${13} color="var(--warning)" />
            <span class="lines-2" title=${tools.updateError}>Couldn't update Purrloader's helper tools. Downloads still work; some sites may not.</span>
            <span class="spacer" />
            ${tools.isUpdating || busy
                ? html`<${Spinner} size="small" />`
                : html`<${SecondaryButton}
                      onClick=${async () => {
                          setBusy(true)
                          await invoke('tools.update')
                          setBusy(false)
                      }}
                      >Try Again<//
                  >`}
            <${CircleButton} icon="xmark" size=${22} filled=${false} label="Dismiss" onClick=${() => invoke('tools.dismissUpdateError')} />`
    } else if (updater.available) {
        content = html`<${Icon} name="sparkles" size=${13} color="var(--secondary)" />
            <span style=${{ color: 'var(--text-90)' }}>Purrloader ${updater.available.version} is available.</span>
            <span class="spacer" />
            <${SecondaryButton} onClick=${() => invoke('openExternal', updater.available.page)}>View<//>
            <${CircleButton} icon="xmark" size=${22} filled=${false} label="Dismiss" onClick=${() => invoke('updater.dismiss')} />`
    }
    if (!content) return null
    return html`<div class="notice-wrap"><div class="notice" role="status">${content}</div></div>`
}

// MARK: - Link field

function UrlField({ model }) {
    const input = useRef(null)
    const [focused, setFocused] = useState(false)
    const [text, setText] = useState(model.urlText)
    useEffect(() => setText(model.urlText), [model.urlRevision])
    useEffect(() => {
        // Right after launch the window isn't focused yet.
        const timer = setTimeout(() => input.current?.focus(), 150)
        return () => clearTimeout(timer)
    }, [])
    const change = (value) => {
        setText(value)
        invoke('model.setUrlText', value)
    }
    return html`<div class=${cx('url-field', focused && 'focused')}>
        <${Icon} name="link" size=${15} weight="medium" color="var(--tertiary)" />
        <input
            ref=${input}
            type="text"
            value=${text}
            placeholder="Paste a link, or drop links and .txt files"
            aria-label="Link"
            spellcheck=${false}
            onInput=${(event) => change(event.target.value)}
            onFocus=${() => setFocused(true)}
            onBlur=${() => setFocused(false)}
            onKeyDown=${(event) => {
                if (event.key === 'Enter' && !isShortcut(event)) invoke(model.info ? 'model.download' : 'model.refetch')
            }}
        />
        ${text &&
        html`<${CircleButton}
            icon="xmark"
            size=${22}
            label="Clear link"
            onClick=${() => {
                change('')
                input.current?.focus()
            }}
        />`}
    </div>`
}

function LinkStatus({ state }) {
    const { model } = state
    if (model.isFetching) {
        return html`<div class="link-status"><${Spinner} size="small" /><span>Getting video info…</span></div>`
    }
    if (model.fetchError) return html`<${FetchErrorRow} error=${model.fetchError} state=${state} />`
    if (model.info) return html`<${InfoCard} info=${model.info} model=${model} />`
    if (model.linkCount > 1) {
        return html`<div class="link-status">
            <${Icon} name="listBullet" size=${12} />
            <span>${model.linkCount} links will be added to the queue</span>
        </div>`
    }
    return null
}

/** A link that couldn't be read, with the button most likely to fix it. */
function FetchErrorRow({ error, state }) {
    const [updating, setUpdating] = useState(false)
    const [picking, setPicking] = useState(false)
    const button = useRef(null)
    const fix = fixFor(error)
    const perform = async () => {
        switch (fix) {
            case 'browserSignIn':
                return setPicking(true)
            case 'updateAndRetry':
                setUpdating(true)
                await invoke('tools.update')
                setUpdating(false)
                return invoke('model.refetch')
            case 'waitForStart':
                return invoke('model.downloadWhenStreamStarts')
            default:
                return invoke('model.refetch')
        }
    }
    return html`<div class="row" style=${{ gap: '10px', paddingLeft: '4px' }}>
        <${Icon} name="exclamationTriangleFill" size=${14} color="var(--warning)" />
        <span class="lines-3 selectable" style=${{ fontSize: '12px', fontWeight: 500, color: 'rgba(255,255,255,0.85)' }}>${error.message}</span>
        <span class="spacer" style=${{ minWidth: '8px' }} />
        ${updating ? html`<${Spinner} size="small" />` : fix && html`<${SecondaryButton} buttonRef=${button} onClick=${perform}>${FIX_TITLES[fix]}<//>`}
        ${picking &&
        html`<${Popover} anchor=${button.current} onClose=${() => setPicking(false)} width=${280}>
            <${BrowserPicker}
                app=${state.app}
                current=${state.model.options.network.cookieBrowser}
                choose=${(browser) => {
                    setPicking(false)
                    invoke('model.setCookieBrowser', browser, true)
                }}
            />
        <//>`}
    </div>`
}

/** The list behind "Use Browser Sign-In": pick a browser and the download retries with its sign-in. */
function BrowserPicker({ app, current, choose }) {
    const browsers = browserList(app, current)
    return html`<div style=${{ display: 'flex', flexDirection: 'column', gap: '10px', padding: '14px' }}>
        <div style=${{ fontSize: '13px', fontWeight: 600 }}>Sign in with my browser</div>
        <div class="note">Sign in to the site in one of these browsers first. Purrloader then uses that sign-in.</div>
        <div style=${{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            ${browsers.map(
                (browser) => html`<button
                    key=${browser}
                    class="row"
                    style=${{ height: '28px', padding: '0 10px', borderRadius: '7px', background: 'rgba(255,255,255,0.05)', fontSize: '13px' }}
                    onClick=${() => choose(browser)}
                >
                    <span class="spacer" style=${{ textAlign: 'left' }}>${CookieBrowser[browser]}</span>
                    ${browser === current && html`<${Icon} name="checkmark" size=${12} weight="semibold" color="var(--secondary)" />`}
                </button>`,
            )}
        </div>
        <div class="note" style=${{ color: 'var(--tertiary)' }}>${app.browserNote}</div>
    </div>`
}

function browserList(app, current) {
    const installed = app.installedBrowsers
    return current === 'none' || installed.includes(current) ? installed : [...installed, current]
}

function InfoCard({ info, model }) {
    const parts = []
    if (info.uploader) parts.push(info.uploader)
    if (info.isPlaylist) parts.push(info.entries.length === 1 ? '1 item' : `${info.entries.length} items`)
    const duration = Format.duration(info.duration)
    if (duration) parts.push(duration)
    return html`<div class="card">
        <div class="info-row">
            <${Thumbnail} url=${info.thumbnail} width=${120} />
            <div class="info-text">
                <div class="info-title lines-2 selectable">${info.title}</div>
                <div class="ellipsis" style=${{ fontSize: '12px', color: 'var(--secondary)', whiteSpace: 'pre' }}>${parts.join('  ·  ')}</div>
                ${(info.isLive || model.options.format.exactFormat) &&
                html`<div class="row" style=${{ gap: '6px' }}>
                    ${info.isLive && html`<${Tag} text="LIVE" color="var(--red)" />`}
                    ${model.options.format.exactFormat && html`<${Tag} text="Custom format" color="#fff" />`}
                </div>`}
            </div>
        </div>
        ${info.isPlaylist && info.entries.length > 0 && html`<${PlaylistPicker} info=${info} selected=${model.selectedEntries} />`}
    </div>`
}

function PlaylistPicker({ info, selected }) {
    const chosen = new Set(selected)
    return html`<div class="playlist">
        <div class="playlist-head">
            <span class="secondary spacer">${chosen.size} of ${info.entries.length} selected</span>
            <button class="plain-button" onClick=${() => invoke('model.selectEntries', true)}>Select All</button>
            <button class="plain-button" onClick=${() => invoke('model.selectEntries', false)}>None</button>
        </div>
        <div class="playlist-rows" style=${{ height: Math.min(info.entries.length * 30, 170) + 'px' }}>
            ${info.entries.map((entry) => {
                const on = chosen.has(entry.index)
                const duration = Format.duration(entry.duration)
                return html`<button
                    key=${entry.index}
                    class=${cx('playlist-row', on && 'selected')}
                    role="checkbox"
                    aria-checked=${on}
                    aria-label=${entry.title}
                    onClick=${() => invoke('model.toggleEntry', entry.index)}
                >
                    <${Icon} name=${on ? 'checkmarkCircleFill' : 'circle'} size=${14} weight="regular" color=${on ? '#fff' : 'var(--tertiary)'} />
                    <span class="index mono-digits">${entry.index}</span>
                    <span class="ellipsis spacer">${entry.title}</span>
                    ${duration && html`<span class="mono-digits tertiary">${duration}</span>`}
                </button>`
            })}
        </div>
    </div>`
}

// MARK: - Controls row

function Controls({ state }) {
    const { model, app } = state
    const format = model.options.format
    const moreRef = useRef(null)
    const [showMore, setShowMore] = useState(false)
    const closeMore = useCallback(() => setShowMore(false), [])

    const heights = [...model.qualityHeights]
    if (format.maxHeight != null && !heights.includes(format.maxHeight)) heights.push(format.maxHeight)
    heights.sort((a, b) => b - a)
    const qualityItems = [
        { label: 'Best', checked: format.maxHeight == null, action: () => invoke('model.setOption', 'format.maxHeight', null) },
        ...heights.map((height) => ({
            label: Format.heightLabel(height),
            checked: format.maxHeight === height,
            action: () => invoke('model.setOption', 'format.maxHeight', height),
        })),
    ]
    const audioChoices = AUDIO_CHOICES.includes(format.audioFormat) ? AUDIO_CHOICES : [...AUDIO_CHOICES, format.audioFormat]
    const audioItems = audioChoices.map((value) => ({
        label: AudioFormat[value],
        checked: format.audioFormat === value,
        action: () => invoke('model.setOption', 'format.audioFormat', value),
    }))
    const more = model.activeMoreOptions === 0 ? 'More' : `More · ${model.activeMoreOptions}`

    return html`<div class="controls">
        <${PillSegment}
            label="Download as"
            value=${format.mode}
            options=${[
                ['video', 'Video'],
                ['audio', 'Audio'],
            ]}
            onChange=${(mode) => invoke('model.setOption', 'format.mode', mode)}
        />
        ${format.mode === 'video'
            ? html`<${ChipMenu}
                  name="Quality"
                  text=${format.maxHeight != null ? Format.heightLabel(format.maxHeight) : 'Best'}
                  items=${qualityItems}
                  title="Highest quality to download"
              />`
            : html`<${ChipMenu}
                  name="Audio format"
                  text=${AudioFormat[format.audioFormat]}
                  items=${audioItems}
                  title="MP3 plays everywhere. Best quality keeps the original sound without converting it."
              />`}
        <${ChipButton}
            text=${model.folderName}
            icon="folder"
            title=${`Save to ${model.folder}`}
            label=${`Save to ${model.folderName}`}
            onClick=${() => invoke('model.chooseFolder')}
        />
        <span class="spacer" />
        <${ChipButton}
            buttonRef=${moreRef}
            text=${more}
            icon="sliders"
            open=${showMore}
            title="Subtitles, sponsor skipping, part of a video, browser sign-in"
            label="More options"
            onClick=${() => setShowMore((open) => !open)}
        />
        ${showMore && html`<${Popover} anchor=${moreRef.current} onClose=${closeMore} width=${330}><${MoreOptions} model=${model} app=${app} close=${closeMore} /><//>`}
        <${PrimaryButton} title=${`Download (${shortcutLabel('Ctrl+Enter')})`} disabled=${!model.canDownload} onClick=${() => invoke('model.download')}>Download<//>
    </div>`
}

/** The few extras a normal download might need. Everything else lives in Settings > Advanced. */
function MoreOptions({ model, app, close }) {
    const options = model.options
    const browsers = browserList(app, options.network.cookieBrowser)
    const section = (children) => html`<div class="more-section">${children}</div>`
    const switchRow = (title, on, action) => html`<div class="row">
        <span class="spacer">${title}</span>
        <${Switch} on=${on} label=${title} onChange=${(value) => invoke(action, value)} />
    </div>`
    return html`<div class="more">
        ${section(html`
            ${switchRow('Subtitles', options.subtitles.enabled, 'model.setSubtitlesOn')}
            ${options.subtitles.enabled &&
            html`<div class="row">
                    <span class="spacer">Language</span>
                    <${PopUpButton}
                        label="Language"
                        value=${model.subtitleSelection}
                        options=${model.subtitleChoices.map((track) => ({ value: track.code, title: track.title }))}
                        onChange=${(code) => invoke('model.setSubtitleSelection', code)}
                    />
                </div>
                <div class="note">${options.format.mode === 'video' ? 'Added to the video.' : 'Saved next to the audio file.'}</div>`}
        `)}
        ${section(html`
            ${switchRow('Skip sponsor segments', model.skipSponsors, 'model.setSkipSponsors')}
            <div class="note">Cuts out sponsor spots, self-promotion and reminders to subscribe. Works on supported sites only.</div>
        `)}
        ${section(html`
            ${switchRow('Download only a part', model.downloadPart, 'model.setDownloadPart')}
            ${model.downloadPart &&
            html`<div class="row">
                    <${TextInput}
                        class="spacer"
                        label="Start"
                        placeholder="0:00"
                        value=${options.extras.sectionStart}
                        revision=${model.optionsRevision}
                        onChange=${(value) => invoke('model.setOption', 'extras.sectionStart', value)}
                    />
                    <span class="secondary">to</span>
                    <${TextInput}
                        class="spacer"
                        label="End"
                        placeholder="End"
                        value=${options.extras.sectionEnd}
                        revision=${model.optionsRevision}
                        onChange=${(value) => invoke('model.setOption', 'extras.sectionEnd', value)}
                    />
                </div>
                <div class="note">Times like 1:30 or 1:02:45.</div>`}
        `)}
        ${section(html`
            <div class="row">
                <span class="spacer">Sign in with my browser</span>
                <${PopUpButton}
                    label="Sign in with my browser"
                    value=${options.network.cookieBrowser}
                    options=${[
                        { value: 'none', title: "Don't sign in" },
                        { separator: true },
                        ...browsers.map((browser) => ({ value: browser, title: CookieBrowser[browser] })),
                    ]}
                    onChange=${(browser) => invoke('model.setCookieBrowser', browser, true)}
                />
            </div>
            <div class="note">For private, age-restricted or members-only content you can watch in that browser.</div>
        `)}
        <div style=${{ padding: '14px' }}>
            <button
                class="link-button"
                onClick=${() => {
                    close()
                    invoke('window.openSettings', 'advanced')
                }}
            >
                Advanced Options…
            </button>
        </div>
    </div>`
}

// MARK: - Queue

function QueueView({ jobs, app, showDetails }) {
    const all = jobs.jobs
    if (!all.length) {
        return html`<div class="queue">
            <div class="empty">
                <div class="empty-ring"><${Icon} name="arrowDown" size=${20} weight="medium" /></div>
                <div style=${{ fontSize: '14px', fontWeight: 600, color: 'var(--secondary)' }}>No downloads yet</div>
                <div style=${{ fontSize: '12px', color: 'var(--tertiary)' }}>Paste a link above, or drop one anywhere in this window.</div>
            </div>
        </div>`
    }
    const current = all.filter((job) => !job.isRecent)
    const recent = all.filter((job) => job.isRecent)
    return html`<div class="queue">
        <div class="queue-list">
            ${current.map((job) => html`<${JobRow} key=${job.id} job=${job} app=${app} showDetails=${showDetails} />`)}
            ${recent.length > 0 &&
            html`<div class="recent-head" style=${{ marginTop: current.length ? '10px' : 0 }}>
                <span class="spacer" role="heading" aria-level="2" style=${{ fontWeight: 600, color: 'var(--tertiary)' }}>Recent</span>
                <button
                    class="plain-button"
                    style=${{ color: 'var(--secondary)' }}
                    title=${`Remove finished downloads from this list. The files stay where they are. (${shortcutLabel('Ctrl+K')})`}
                    onClick=${() => invoke('jobs.clearRecent')}
                >
                    Clear
                </button>
            </div>`}
            ${recent.map((job) => html`<${JobRow} key=${job.id} job=${job} app=${app} showDetails=${showDetails} />`)}
        </div>
    </div>`
}

const STATUS_ICONS = {
    finished: ['checkmarkCircleFill', 'var(--green)'],
    failed: ['exclamationCircleFill', 'var(--red)'],
    paused: ['pauseCircleFill', null],
    cancelled: ['xmarkCircleFill', null],
}

function JobRow({ job, app, showDetails }) {
    const [updating, setUpdating] = useState(false)
    const [picking, setPicking] = useState(false)
    const fixButton = useRef(null)
    const reveal = `Show in ${app.fileManager}`

    const icon = (name, label, action) => html`<${CircleButton} icon=${name} size=${28} label=${label} onClick=${(event) => {
        event.stopPropagation()
        action()
    }} />`
    const run = (action, ...args) => () => invoke(action, job.id, ...args)

    const fix = job.status === 'failed' ? fixFor(job.error) : null
    const performFix = async () => {
        switch (fix) {
            case 'browserSignIn':
                return setPicking(true)
            case 'updateAndRetry':
                setUpdating(true)
                await invoke('jobs.updateAndRetry', job.id)
                return setUpdating(false)
            case 'chooseFolder':
                return invoke('jobs.chooseFolderAndRetry', job.id)
            case 'useBestQuality':
                return invoke('jobs.retry', job.id, { bestQuality: true })
            case 'waitForStart':
                return invoke('jobs.retry', job.id, { waitForVideo: true })
            default:
                return invoke('jobs.retry', job.id)
        }
    }

    let buttons
    switch (job.status) {
        case 'queued':
        case 'starting':
        case 'downloading':
        case 'processing':
            buttons = html`${icon('pauseFill', 'Pause', run('jobs.pause'))}${icon('xmark', 'Cancel', run('jobs.cancel'))}`
            break
        case 'paused':
            buttons = html`${icon('playFill', 'Resume', run('jobs.resume'))}${icon('xmark', 'Cancel', run('jobs.cancel'))}`
            break
        case 'failed':
            buttons = html`${updating
                ? html`<span style=${{ padding: '0 8px' }}><${Spinner} size="small" /></span>`
                : fix && html`<${SecondaryButton} buttonRef=${fixButton} onClick=${performFix}>${FIX_TITLES[fix]}<//>`}
            ${fix !== 'retry' && icon('arrowClockwise', 'Retry', run('jobs.retry'))}${icon('trash', 'Remove', run('jobs.remove'))}`
            break
        case 'cancelled':
            buttons = html`${icon('arrowClockwise', 'Download Again', run('jobs.retry'))}${icon('trash', 'Remove', run('jobs.remove'))}`
            break
        case 'finished':
            buttons = html`${icon('magnifyingglass', reveal, run('jobs.reveal'))}${icon('trash', 'Remove from list', run('jobs.remove'))}`
            break
    }

    const contextMenu = (event) => {
        event.preventDefault()
        openMenu({
            anchor: { x: event.clientX, y: event.clientY },
            items: [
                ...(job.status === 'finished' && job.hasFiles ? [{ label: 'Open', action: run('jobs.open') }] : []),
                { label: reveal, action: run('jobs.reveal') },
                { label: 'Copy Link', action: run('jobs.copyLink') },
                { label: 'Show Details', action: () => showDetails(job.id) },
                { separator: true },
                { label: 'Remove from List', disabled: job.isActive, action: run('jobs.remove') },
            ],
        })
    }

    const [statusIcon, statusColor] = STATUS_ICONS[job.status] ?? []
    return html`<div
        class="job"
        role="group"
        aria-label=${job.title}
        aria-description=${job.statusText}
        onContextMenu=${contextMenu}
        onDblClick=${() => (job.status === 'finished' ? invoke('jobs.reveal', job.id) : showDetails(job.id))}
    >
        <div style=${{ opacity: job.status === 'cancelled' ? 0.5 : 1 }}><${Thumbnail} url=${job.thumbnail} width=${job.isRecent ? 64 : 80} /></div>
        <div class="job-text">
            <${MiddleText} class="job-title" text=${job.title} />
            ${job.isActive && html`<${SlimProgress} fraction=${job.fraction} />`}
            <div class="job-status" title=${job.statusText}>
                ${statusIcon && html`<${Icon} name=${statusIcon} size=${12} color=${statusColor} />`}
                <span class=${job.error ? 'lines-2' : 'ellipsis'}>${job.statusText}</span>
                ${job.isActive && job.fraction != null && html`<span class="spacer" /><span style=${{ color: 'rgba(255,255,255,0.8)' }}>${Math.floor(job.fraction * 100)}%</span>`}
            </div>
            ${job.isActive && job.currentItemTitle && html`<div class="ellipsis" style=${{ fontSize: '11px', color: 'var(--tertiary)' }}>${job.currentItemTitle}</div>`}
        </div>
        <div class=${cx('job-buttons', !(job.isActive || job.error) && 'dim')}>
            ${buttons}
            ${icon('textAlignLeft', 'Details', () => showDetails(job.id))}
        </div>
        ${picking &&
        html`<${Popover} anchor=${fixButton.current} onClose=${() => setPicking(false)} width=${280}>
            <${BrowserPicker}
                app=${app}
                current=${job.cookieBrowser}
                choose=${(browser) => {
                    setPicking(false)
                    // Remember it, so the next private video works straight away.
                    invoke('model.setCookieBrowser', browser, false)
                    invoke('jobs.retry', job.id, { cookieBrowser: browser })
                }}
            />
        <//>`}
    </div>`
}

/** What happened during a download: its full log, and the command behind it for the curious. */
function JobDetails({ id, onClose }) {
    const [details, setDetails] = useState(null)
    const [showCommand, setShowCommand] = useState(false)
    const log = useRef(null)
    useEffect(() => {
        let alive = true
        const load = async () => {
            const next = await invoke('jobs.details', id)
            if (alive && next) setDetails(next)
        }
        load()
        const timer = setInterval(load, 500)
        return () => {
            alive = false
            clearInterval(timer)
        }
    }, [id])
    useEffect(() => {
        if (log.current) log.current.scrollTop = log.current.scrollHeight
    }, [details?.log.length])
    if (!details) return null
    return html`<${Sheet} onClose=${onClose} label="Download details">
        <div class="details" style=${{ width: 'min(760px, calc(100vw - 48px))', height: 'min(540px, calc(100vh - 48px))' }}>
            <div style=${{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div class="lines-2" style=${{ fontSize: '15px', fontWeight: 600 }}>${details.title}</div>
                <div class="lines-2" style=${{ fontSize: '12px', color: 'var(--secondary)' }}>${details.statusText}</div>
            </div>
            <div class="log" ref=${log}>
                ${details.log.map(
                    (line, index) =>
                        html`<div key=${index} class=${line.startsWith('ERROR') ? 'error' : line.startsWith('WARNING') ? 'warning' : ''}>${line}</div>`,
                )}
            </div>
            <div style=${{ fontSize: '12px' }}>
                <button class="row" style=${{ gap: '5px' }} aria-expanded=${showCommand} onClick=${() => setShowCommand(!showCommand)}>
                    <${Icon} name="chevronRight" size=${9} weight="bold" style=${{ transform: showCommand ? 'rotate(90deg)' : 'none', transition: 'transform 0.15s' }} />
                    Command
                </button>
                ${showCommand &&
                html`<div class="row" style=${{ alignItems: 'flex-start', paddingTop: '6px' }}>
                    <div class="mono selectable spacer" style=${{ fontSize: '10.5px', lineHeight: 1.45, wordBreak: 'break-all' }}>${details.commandLine}</div>
                    <${CircleButton} icon="docOnDoc" size=${24} filled=${false} label="Copy command" onClick=${() => invoke('copy', details.commandLine)} />
                </div>`}
            </div>
            <div class="row">
                <${BezelButton} onClick=${() => invoke('copy', details.log.join('\n'))}>Copy Log<//>
                <span class="spacer" />
                <${BezelButton} isDefault onClick=${onClose}>Done<//>
            </div>
        </div>
    <//>`
}

// MARK: - Welcome

/** Shown once, on first launch, while the helper tools download. */
function Welcome({ tools, app }) {
    const progress = tools.installProgress
    let status
    if (tools.setupError) {
        status = html`<div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
            <div class="row" style=${{ color: 'var(--warning)', fontSize: '12px', fontWeight: 500, textAlign: 'center' }} title=${tools.setupError}>
                <${Icon} name="exclamationTriangleFill" size=${13} />
                Couldn't download the helper tools. Check your internet connection.
            </div>
            <${SecondaryButton} onClick=${() => invoke('tools.retrySetup')}>Try Again<//>
        </div>`
    } else if (tools.isReady) {
        status = html`<div class="row" style=${{ color: 'var(--green)', fontSize: '12px', fontWeight: 500 }}>
            <${Icon} name="checkmarkCircleFill" size=${13} /> Ready to go.
        </div>`
    } else {
        const caption =
            progress?.expected > 0
                ? `Downloading free helper tools: ${Format.bytes(progress.written)} of ${Format.bytes(progress.expected)}`
                : 'Downloading free helper tools…'
        const fraction = progress?.expected > 0 ? progress.written / progress.expected : 0
        status = html`<div style=${{ width: '100%', display: 'flex', flexDirection: 'column', gap: '6px' }}>
            <div style=${{ height: '6px', borderRadius: '3px', background: 'rgba(255,255,255,0.12)', overflow: 'hidden' }}>
                <div style=${{ height: '100%', width: `${fraction * 100}%`, background: '#fff', borderRadius: '3px', transition: 'width 0.3s' }} />
            </div>
            <div class="mono-digits" style=${{ fontSize: '11px', color: 'var(--secondary)' }}>${caption}</div>
        </div>`
    }
    const where = app.platform === 'darwin' ? 'your Mac' : 'your computer'
    return html`<${Sheet} label="Welcome to Purrloader">
        <div style=${{ width: '440px', padding: '30px 36px', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '18px' }}>
            <img src="/assets/icon.png" width="96" height="96" alt="" />
            <div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '6px', textAlign: 'center' }}>
                <div style=${{ fontSize: '20px', fontWeight: 600, letterSpacing: '-0.02em' }}>Welcome to Purrloader</div>
                <div style=${{ fontSize: '13px', color: 'var(--secondary)', lineHeight: 1.35 }}>
                    Paste a link to a video and Purrloader saves it, or just its audio, to ${where}.
                </div>
            </div>
            <div style=${{ minHeight: '44px', width: '100%', display: 'grid', placeItems: 'center' }}>${status}</div>
            <div style=${{ fontSize: '11px', color: 'var(--tertiary)', textAlign: 'center' }}>Only download content you own or have permission to download.</div>
            <${PrimaryButton} onClick=${() => invoke('settings.set', 'didShowWelcome', true)}>Get Started<//>
        </div>
    <//>`
}
