// The Settings window: General, Advanced (every yt-dlp option Purrloader exposes) and About.
// Ports of Features/Settings in the Mac app.

import {
    BezelButton,
    Checkbox,
    Form,
    PickerRow,
    PillSegment,
    PopUpButton,
    Row,
    Section,
    Segmented,
    Sheet,
    Spinner,
    Stepper,
    SwitchRow,
    TextInput,
    enumOptions,
    openMenu,
} from './controls.js'
import { Icon } from './icons.js'
import { cx, html, invoke, isMac, useAppState, useEffect, useRef, useState } from './lib.js'
import {
    AudioFormat,
    CodecPreference,
    Container,
    ContainerMethod,
    CookieBrowser,
    MediaMode,
    OverwriteRule,
    SPONSOR_CATEGORIES,
    SponsorBlockMode,
    isLossy,
} from '../core/options.js'
import { TEMPLATE_PRESETS } from '../core/media.js'
import { Format } from '../core/support.js'

const TABS = [
    ['general', 'General', 'gearshape'],
    ['advanced', 'Advanced', 'sliders'],
    ['about', 'About', 'infoCircle'],
]

export function SettingsWindow() {
    const state = useAppState()
    const tab = TABS.some(([id]) => id === state.settings.settingsTab) ? state.settings.settingsTab : 'general'
    useEffect(() => {
        document.title = TABS.find(([id]) => id === tab)[1]
        invoke('window.resizeSettings', tab)
    }, [tab])
    return html`<div class=${cx('window', isMac && 'mac')}>
        <div class="toolbar" role="tablist">
            ${TABS.map(
                ([id, title, icon]) => html`<button
                    key=${id}
                    class=${cx('tab', id === tab && 'selected')}
                    role="tab"
                    aria-selected=${id === tab}
                    onClick=${() => invoke('settings.set', 'settingsTab', id)}
                >
                    <${Icon} name=${icon} size=${19} weight="regular" />
                    <span>${title}</span>
                </button>`,
            )}
        </div>
        ${tab === 'general' && html`<div class="settings-scroll"><${GeneralSettings} state=${state} /></div>`}
        ${tab === 'advanced' && html`<${AdvancedSettings} state=${state} />`}
        ${tab === 'about' && html`<div class="settings-scroll"><${AboutSettings} state=${state} /></div>`}
    </div>`
}

const set = (keyPath) => (value) => invoke('model.setOption', keyPath, value)

/** The path the way people know it: ~ for the home folder on Linux and the Mac. */
function displayPath(path, app) {
    if (app.platform === 'win32') return path
    return path.replace(/^\/(home|Users)\/[^/]+/, '~')
}

// MARK: - General

function GeneralSettings({ state }) {
    const { model, settings, app } = state
    return html`<${Form}>
        <${Section}>
            <${Row} label="Save downloads to">
                <span class="ellipsis secondary" title=${model.folder} style=${{ direction: 'rtl', textAlign: 'left' }}>
                    <bdi>${displayPath(model.folder, app)}</bdi>
                </span>
                <${BezelButton} onClick=${() => invoke('model.chooseFolder')}>Change…<//>
            <//>
            <${Row} label=${`Download ${settings.maxConcurrent} at a time`}>
                <${Stepper}
                    label="Downloads at a time"
                    value=${settings.maxConcurrent}
                    min=${1}
                    max=${10}
                    onChange=${(value) => invoke('settings.setMaxConcurrent', value)}
                />
            <//>
        <//>
        <${Section}>
            <${SwitchRow}
                label="Fill in links from the clipboard"
                on=${settings.clipboardAutofill}
                onChange=${(on) => invoke('settings.set', 'clipboardAutofill', on)}
            />
            <${SwitchRow}
                label="Notify me when downloads finish or fail"
                on=${settings.notifications}
                onChange=${(on) => invoke('settings.set', 'notifications', on)}
            />
            <${SwitchRow}
                label=${`Show Purrloader in the ${app.trayName}`}
                on=${settings.showTrayIcon}
                onChange=${(on) => invoke('settings.set', 'showTrayIcon', on)}
            />
        <//>
    <//>`
}

// MARK: - Advanced

const SECTIONS = ['Format', 'Playlist', 'Extras', 'Network', 'Output', 'Command']

/** Every yt-dlp option Purrloader exposes, grouped by topic. These apply to every download. */
function AdvancedSettings({ state }) {
    const [section, setSection] = useState('Format')
    const [saving, setSaving] = useState(false)
    const [managing, setManaging] = useState(false)
    const setupsButton = useRef(null)

    const resetToDefaults = async () => {
        const choice = await invoke('confirm', {
            message: 'Reset every advanced setting?',
            detail: 'Video, audio, subtitle, network and file name settings go back to how Purrloader came. Saved setups are kept.',
            buttons: ['Reset to Defaults', 'Cancel'],
        })
        if (choice === 0) invoke('model.resetOptions')
    }
    const openSetups = () =>
        openMenu({
            anchor: setupsButton.current,
            items: [
                ...state.presets.map((preset) => ({ label: preset.name, action: () => invoke('presets.apply', preset.id) })),
                { separator: true },
                { label: 'Save Current Settings…', action: () => setSaving(true) },
                { label: 'Manage Saved Setups…', action: () => setManaging(true) },
            ],
        })

    const content = {
        Format: FormatSection,
        Playlist: PlaylistSection,
        Extras: ExtrasSection,
        Network: NetworkSection,
        Output: OutputSection,
        Command: CommandSection,
    }[section]

    return html`<div style=${{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
        <div class="advanced-head">
            <div style=${{ display: 'flex', flexDirection: 'column', gap: '3px' }} class="spacer">
                <div style=${{ fontSize: '13px', fontWeight: 600 }}>These settings apply to every download.</div>
                <div style=${{ fontSize: '12px', color: 'var(--secondary)' }}>You don't need any of them for everyday downloads.</div>
            </div>
            <button ref=${setupsButton} class="bezel-button" style=${{ gap: '5px' }} aria-haspopup="menu" onClick=${openSetups}>
                Saved Setups <${Icon} name="chevronDown" size=${8} weight="bold" />
            </button>
            <${BezelButton} onClick=${resetToDefaults}>Reset to Defaults…<//>
        </div>
        <div style=${{ display: 'flex', justifyContent: 'center', paddingTop: '14px' }}>
            <${PillSegment} label="Section" height=${30} value=${section} options=${SECTIONS.map((s) => [s, s])} onChange=${setSection} />
        </div>
        <div class="settings-scroll"><${content} state=${state} /></div>
        ${saving && html`<${SaveSetupSheet} onClose=${() => setSaving(false)} />`}
        ${managing && html`<${ManageSetupsSheet} presets=${state.presets} onClose=${() => setManaging(false)} />`}
    </div>`
}

function FormatSection({ state }) {
    const { model } = state
    const format = model.options.format
    const qualities = [
        { value: null, title: 'Best available' },
        ...[4320, 2160, 1440, 1080, 720, 480, 360].map((height) => ({ value: height, title: Format.heightLabel(height) })),
    ]
    const audioQualities = [
        { value: '0', title: 'Best (VBR 0)' },
        { value: '2', title: 'High (VBR 2)' },
        { value: '5', title: 'Standard (VBR 5)' },
        { separator: true },
        ...['320K', '256K', '192K', '160K', '128K', '96K', '64K'].map((value) => ({ value, title: value.replace('K', ' kbps') })),
    ]
    return html`<${Form}>
        <${Section}>
            <${Row} label="Download">
                <${Segmented} label="Download" value=${format.mode} options=${Object.entries(MediaMode)} onChange=${set('format.mode')} />
            <//>
        <//>
        ${format.mode === 'video'
            ? html`<${Section} header="Video">
                  <${PickerRow} label="Maximum quality" value=${format.maxHeight} options=${qualities} onChange=${set('format.maxHeight')} />
                  <${PickerRow} label="Codec preference" value=${format.codec} options=${enumOptions(CodecPreference)} onChange=${set('format.codec')} />
                  <${SwitchRow} label="Allow HDR" on=${format.allowHDR} onChange=${set('format.allowHDR')} />
                  <${PickerRow} label="Container" value=${format.container} options=${enumOptions(Container)} onChange=${set('format.container')} />
                  ${format.container !== 'auto' &&
                  html`<${PickerRow}
                      label="Conversion"
                      value=${format.containerMethod}
                      options=${enumOptions(ContainerMethod)}
                      onChange=${set('format.containerMethod')}
                  />`}
              <//>`
            : html`<${Section} header="Audio">
                  <${PickerRow} label="Format" value=${format.audioFormat} options=${enumOptions(AudioFormat)} onChange=${set('format.audioFormat')} />
                  ${isLossy(format.audioFormat) &&
                  html`<${PickerRow} label="Quality" value=${format.audioQuality} options=${audioQualities} onChange=${set('format.audioQuality')} />`}
              <//>`}
        <${Section}
            header="Exact streams"
            footer="Pick specific streams from the current link's format list (same as yt-dlp -F). Overrides the quality settings above for that link only."
        >
            <${FormatTable} model=${model} />
        <//>
    <//>`
}

function FormatTable({ model }) {
    const info = model.info
    const exact = model.options.format.exactFormat
    if (!info || !info.formats.length) {
        return html`<${Row} stack>
            <div class="secondary">
                ${info?.isPlaylist ? 'Stream picking is available for single videos.' : 'Paste a link in the main window to see its formats.'}
            </div>
            <${TextInput}
                class="mono"
                label="Format code"
                placeholder="e.g. 137+140 or bv*+ba/b"
                value=${exact}
                revision=${model.optionsRevision}
                onChange=${set('format.exactFormat')}
            />
        <//>`
    }
    const picked = exact.split('+').filter(Boolean)
    const byId = new Map(info.formats.map((f) => [f.id, f]))
    let video = picked.find((id) => byId.get(id)?.hasVideo) ?? null
    let audio = picked.find((id) => byId.has(id) && !byId.get(id).hasVideo) ?? null
    // One video (or combined) stream plus one audio-only stream at most.
    const toggle = (format) => {
        if (format.id === video) video = null
        else if (format.id === audio) audio = null
        else if (format.hasVideo) {
            video = format.id
            if (format.hasAudio) audio = null
        } else audio = format.id
        const next = [video, audio].filter(Boolean).join('+')
        invoke('model.setExactFormat', next, video ? 'video' : audio ? 'audio' : null)
    }
    const codec = (f) => (f.hasVideo ? (f.vcodec ?? '') + ((f.dynamicRange ?? 'SDR') === 'SDR' ? '' : ' ' + f.dynamicRange) : '—')
    return html`<${Row} stack>
        <div class="table-wrap">
            <table class="format-table">
                <thead>
                    <tr><th>ID</th><th>Ext</th><th>Resolution</th><th>Video</th><th>Audio</th><th>Bitrate</th><th>Size</th><th>Note</th></tr>
                </thead>
                <tbody>
                    ${info.formats.map(
                        (f) => html`<tr key=${f.id} class=${cx((f.id === video || f.id === audio) && 'selected')} onClick=${() => toggle(f)}>
                            <td>${f.id}</td>
                            <td>${f.ext}</td>
                            <td>${f.resolution}</td>
                            <td>${codec(f)}</td>
                            <td>${f.hasAudio ? f.acodec : '—'}</td>
                            <td>${f.bitrate != null ? `${Math.trunc(f.bitrate)}k` : ''}</td>
                            <td>${f.size != null ? Format.bytes(f.size) : ''}</td>
                            <td>${f.note}</td>
                        </tr>`,
                    )}
                </tbody>
            </table>
        </div>
        <div class="row" style=${{ paddingTop: '8px' }}>
            <span class="mono secondary spacer" style=${{ fontSize: '12px' }}>${exact ? `-f ${exact}` : 'Automatic (no streams picked)'}</span>
            <${BezelButton} disabled=${!exact} onClick=${() => invoke('model.setExactFormat', '', null)}>Clear<//>
        </div>
    <//>`
}

function PlaylistSection({ state }) {
    const { model, app } = state
    const playlist = model.options.playlist
    const [archiveCount, setArchiveCount] = useState(0)
    useEffect(() => {
        invoke('archive.count').then((count) => setArchiveCount(count ?? 0))
    }, [])
    const clearArchive = async () => {
        const choice = await invoke('confirm', {
            message: 'Clear the download archive?',
            detail: 'Items you already downloaded will be downloaded again next time.',
            buttons: ['Clear Archive', 'Cancel'],
        })
        if (choice === 0) {
            await invoke('archive.clear')
            setArchiveCount(0)
        }
    }
    return html`<${Form}>
        <${Section} footer="For links that point to one video inside a playlist.">
            <${SwitchRow} label="Only download the video, not the playlist it's in" on=${playlist.noPlaylist} onChange=${set('playlist.noPlaylist')} />
        <//>
        <${Section} header="Which items">
            <${Row} label="Range">
                <${TextInput}
                    class="plain spacer"
                    label="Range"
                    placeholder="All — or e.g. 5-20, or 1,3,7:10"
                    value=${playlist.range}
                    revision=${model.optionsRevision}
                    onChange=${set('playlist.range')}
                    style=${{ width: '260px' }}
                />
            <//>
            <${Row} label="Latest">
                <span class="secondary">${playlist.latestCount === 0 ? 'All' : `${playlist.latestCount} items`}</span>
                <${Stepper} label="Latest" value=${playlist.latestCount} min=${0} max=${10000} onChange=${set('playlist.latestCount')} />
            <//>
            <${SwitchRow} label="Reverse order" on=${playlist.reverse} onChange=${set('playlist.reverse')} />
        <//>
        <${Section}
            header="Download archive"
            footer="Records every downloaded video ID so re-running a playlist or channel only fetches new items."
        >
            <${SwitchRow} label="Skip items already downloaded" on=${playlist.useArchive} onChange=${set('playlist.useArchive')} />
            <${Row} label="Archive">
                <span class="secondary">${archiveCount === 1 ? '1 item' : `${archiveCount} items`}</span>
                <${BezelButton} disabled=${archiveCount === 0} onClick=${() => invoke('archive.show')}>Show<//>
                <${BezelButton} disabled=${archiveCount === 0} onClick=${clearArchive}>Clear…<//>
            <//>
        <//>
        ${model.info?.isPlaylist &&
        html`<${Section}>
            <${Row}>
                <span class="secondary">
                    Items ticked in the main window take priority over the range above (${model.selectedEntries.length} of
                    ${model.info.entries.length} ticked).
                </span>
            <//>
        <//>`}
    <//>`
}

function ExtrasSection({ state }) {
    const { model } = state
    const { subtitles, extras } = model.options
    const toggleCategory = (category, on) => {
        const next = extras.sponsorCategories.filter((c) => c !== category)
        if (on) next.push(category)
        set('extras.sponsorCategories')(next)
    }
    return html`<${Form}>
        <${Section} header="Subtitles">
            <${SwitchRow} label="Download subtitles" on=${subtitles.enabled} onChange=${set('subtitles.enabled')} />
            <${SwitchRow} label="Include auto-generated subtitles" on=${subtitles.includeAutoGenerated} onChange=${set('subtitles.includeAutoGenerated')} />
            ${subtitles.enabled &&
            html`<${Row} label="Languages">
                    <${TextInput}
                        class="plain"
                        label="Languages"
                        placeholder="en.*,de — or all,-live_chat"
                        value=${subtitles.languages}
                        revision=${model.optionsRevision}
                        onChange=${set('subtitles.languages')}
                        style=${{ width: '240px' }}
                    />
                <//>
                <${PickerRow}
                    label="Save as"
                    value=${subtitles.embed}
                    options=${[
                        { value: true, title: 'Embed in video' },
                        { value: false, title: 'Separate files' },
                    ]}
                    onChange=${set('subtitles.embed')}
                />
                <${SwitchRow} label="Convert to SRT" on=${subtitles.convertToSRT} onChange=${set('subtitles.convertToSRT')} />`}
        <//>
        <${Section} header="Thumbnail & metadata" footer="Cover art and chapters are only embedded in formats that can hold them (not WebM or WAV).">
            <${SwitchRow} label="Embed thumbnail as cover art" on=${extras.embedThumbnail} onChange=${set('extras.embedThumbnail')} />
            <${SwitchRow} label="Save thumbnail as a file" on=${extras.writeThumbnail} onChange=${set('extras.writeThumbnail')} />
            <${SwitchRow} label="Embed metadata" on=${extras.embedMetadata} onChange=${set('extras.embedMetadata')} />
            <${SwitchRow} label="Embed chapters" on=${extras.embedChapters} onChange=${set('extras.embedChapters')} />
            <${SwitchRow} label="Split into one file per chapter" on=${extras.splitChapters} onChange=${set('extras.splitChapters')} />
        <//>
        <${Section} header="SponsorBlock">
            <${PickerRow} label="Segments" value=${extras.sponsorBlock} options=${enumOptions(SponsorBlockMode)} onChange=${set('extras.sponsorBlock')} />
            ${extras.sponsorBlock !== 'off' &&
            html`<${Row} stack>
                <div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))', gap: '8px' }}>
                    ${SPONSOR_CATEGORIES.map(
                        (category) => html`<${Checkbox}
                            key=${category}
                            label=${category.replace('_', ' ').replace(/\b\w/g, (c) => c.toUpperCase())}
                            on=${extras.sponsorCategories.includes(category)}
                            onChange=${(on) => toggleCategory(category, on)}
                        />`,
                    )}
                </div>
            <//>`}
        <//>
        <${Section} header="Only part of the video" footer="Start and end times are set per download in the main window's More menu.">
            <${Row} label="Chapters">
                <${TextInput}
                    class="plain"
                    label="Chapters"
                    placeholder="Chapter titles matching (regex), e.g. intro|verse"
                    value=${extras.chapterRegex}
                    revision=${model.optionsRevision}
                    onChange=${set('extras.chapterRegex')}
                    style=${{ width: '320px' }}
                />
            <//>
            <${SwitchRow} label="Precise cuts (re-encodes around cut points)" on=${extras.forceKeyframes} onChange=${set('extras.forceKeyframes')} />
        <//>
        <${Section} header="Live streams">
            <${SwitchRow} label="Record from the start" on=${extras.liveFromStart} onChange=${set('extras.liveFromStart')} />
            <${SwitchRow} label="Wait for scheduled streams" on=${extras.waitForVideo} onChange=${set('extras.waitForVideo')} />
            ${extras.waitForVideo &&
            html`<${Row} label=${`Check every ${extras.waitInterval} s`}>
                <${Stepper} label="Check interval" value=${extras.waitInterval} min=${5} max=${3600} step=${15} onChange=${set('extras.waitInterval')} />
            <//>`}
        <//>
        <${Section} header="Also save">
            <${SwitchRow} label="Description" on=${extras.writeDescription} onChange=${set('extras.writeDescription')} />
            <${SwitchRow} label="Comments (in the info JSON)" on=${extras.writeComments} onChange=${set('extras.writeComments')} />
            <${SwitchRow} label="Info JSON" on=${extras.writeInfoJSON} onChange=${set('extras.writeInfoJSON')} />
        <//>
    <//>`
}

function aria2cHint(platform) {
    if (platform === 'win32') return 'aria2c not found. Install it with `winget install aria2.aria2`.'
    if (platform === 'darwin') return 'aria2c not found. Install it with `brew install aria2`.'
    return 'aria2c not found. Install it with your package manager, e.g. `sudo apt install aria2`.'
}

function NetworkSection({ state }) {
    const { model, app } = state
    const network = model.options.network
    const [aria2c, setAria2c] = useState(undefined)
    useEffect(() => {
        invoke('network.findAria2c').then(setAria2c)
    }, [])
    const browsers = ['none', ...app.supportedBrowsers]
    if (!browsers.includes(network.cookieBrowser)) browsers.push(network.cookieBrowser)
    const field = (label, keyPath, placeholder, width = 260) => html`<${Row} label=${label}>
        <${TextInput}
            class="plain"
            label=${label}
            placeholder=${placeholder}
            value=${keyPath.split('.').reduce((o, k) => o[k], model.options)}
            revision=${model.optionsRevision}
            onChange=${set(keyPath)}
            style=${{ width: width + 'px' }}
        />
    <//>`
    return html`<${Form}>
        <${Section}
            header="Sign-in"
            footer=${network.cookieBrowser === 'arc' && app.platform === 'darwin'
                ? "yt-dlp has no built-in Arc support; Purrloader reads Arc's profile as Chromium, which may fail to decrypt. If it does, export a cookies.txt from Arc instead."
                : `For private, age-restricted or members-only videos. ${app.browserNote}`}
        >
            <${PickerRow}
                label="Cookies from browser"
                value=${network.cookieBrowser}
                options=${browsers.map((browser) => ({ value: browser, title: CookieBrowser[browser] }))}
                onChange=${(browser) => invoke('model.setCookieBrowser', browser, false)}
            />
            ${network.cookieBrowser !== 'none'
                ? field('Profile', 'network.cookieProfile', 'Default profile')
                : html`<${Row} label="Cookies file">
                      <${TextInput}
                          class="plain"
                          label="Cookies file"
                          placeholder="cookies.txt (Netscape format)"
                          value=${network.cookiesFile}
                          revision=${model.optionsRevision}
                          onChange=${set('network.cookiesFile')}
                          style=${{ width: '220px' }}
                      />
                      <${BezelButton} onClick=${() => invoke('network.chooseCookiesFile')}>Choose…<//>
                  <//>`}
        <//>
        <${Section} header="Connection">
            ${field('Proxy', 'network.proxy', 'socks5://127.0.0.1:1080')}
            ${field('Rate limit', 'network.rateLimit', 'Unlimited — or e.g. 5M')}
            <${Row} label=${`Retries: ${network.retries}`}>
                <${Stepper} label="Retries" value=${network.retries} min=${0} max=${100} onChange=${set('network.retries')} />
            <//>
            <${Row} label=${`Concurrent fragments: ${network.concurrentFragments}`}>
                <${Stepper}
                    label="Concurrent fragments"
                    value=${network.concurrentFragments}
                    min=${1}
                    max=${32}
                    onChange=${set('network.concurrentFragments')}
                />
            <//>
            <${SwitchRow}
                label="Use aria2c as external downloader"
                sub=${aria2c === null ? aria2cHint(app.platform) : null}
                on=${network.useAria2c}
                disabled=${aria2c === null && !network.useAria2c}
                onChange=${set('network.useAria2c')}
            />
        <//>
        <${Section} header="Identity">
            <${PickerRow}
                label="Impersonate browser"
                value=${network.impersonate}
                options=${[{ value: '', title: 'Off' }, ...['chrome', 'edge', 'safari', 'firefox'].map((v) => ({ value: v, title: v[0].toUpperCase() + v.slice(1) }))]}
                onChange=${set('network.impersonate')}
            />
            ${field('Geo bypass (X-Forwarded-For)', 'network.xff', 'Off — or a country code like US, or default')}
            ${field('User agent', 'network.userAgent', 'yt-dlp default')}
        <//>
    <//>`
}

function OutputSection({ state }) {
    const { model } = state
    const output = model.options.output
    const presetsButton = useRef(null)
    return html`<${Form}>
        <${Section} header="File name">
            <${Row} stack>
                <div class="row">
                    <${TextInput}
                        class="mono spacer"
                        label="Template"
                        value=${output.template}
                        revision=${model.optionsRevision}
                        onChange=${set('output.template')}
                    />
                    <button
                        ref=${presetsButton}
                        class="bezel-button"
                        style=${{ gap: '5px' }}
                        aria-haspopup="menu"
                        onClick=${() =>
                            openMenu({
                                anchor: presetsButton.current,
                                align: 'right',
                                items: TEMPLATE_PRESETS.map(([name, template]) => ({
                                    label: name,
                                    action: () => invoke('model.setOption', 'output.template', template),
                                })),
                            })}
                    >
                        Presets <${Icon} name="chevronDown" size=${8} weight="bold" />
                    </button>
                </div>
            <//>
            <${Row} label="Preview">
                <span class="mono selectable" style=${{ fontSize: '12px', textAlign: 'right', wordBreak: 'break-all' }}>${model.templatePreview}</span>
            <//>
            <${Row}>
                <button class="link-button" style=${{ fontSize: '12px' }} onClick=${() => invoke('openExternal', 'https://github.com/yt-dlp/yt-dlp#output-template')}>
                    Template fields reference
                </button>
            <//>
        <//>
        <${Section} header="Files">
            <${SwitchRow} label="Restrict file names to ASCII, no spaces" on=${output.restrictFilenames} onChange=${set('output.restrictFilenames')} />
            <${PickerRow} label="Existing files" value=${output.overwrite} options=${enumOptions(OverwriteRule)} onChange=${set('output.overwrite')} />
        <//>
    <//>`
}

function CommandSection({ state }) {
    const { model, app } = state
    const configPath = app.platform === 'win32' ? '%APPDATA%\\yt-dlp\\config.txt' : '~/.config/yt-dlp/config'
    return html`<${Form}>
        <${Section}
            header="Extra yt-dlp arguments"
            footer="Added after everything else, so they override the options in the other sections. Shell-style quoting works."
        >
            <${Row} stack>
                <${TextInput}
                    multiline
                    rows=${3}
                    class="mono"
                    label="Extra arguments"
                    placeholder='e.g. --match-filters "duration < 600" --sleep-interval 5'
                    value=${model.options.extraArguments}
                    revision=${model.optionsRevision}
                    onChange=${(value) => invoke('model.setOption', 'extraArguments', value)}
                />
            <//>
        <//>
        <${Section} footer=${`Keeps ${configPath} from changing Purrloader's output and progress handling.`}>
            <${SwitchRow}
                label="Ignore my yt-dlp config files"
                on=${model.options.ignoreConfig}
                onChange=${(on) => invoke('model.setOption', 'ignoreConfig', on)}
            />
        <//>
        <${Section}
            header="Command preview"
            footer="Exactly what the Download button will run for the current link. Each download's full log is under Details in the queue."
        >
            <${Row} stack>
                <div class="mono selectable" style=${{ fontSize: '10.5px', lineHeight: 1.45, wordBreak: 'break-all' }}>${model.previewCommand}</div>
                <div class="row" style=${{ justifyContent: 'flex-end', paddingTop: '8px' }}>
                    <${BezelButton} onClick=${() => invoke('copy', model.previewCommand)}>Copy Command<//>
                </div>
            <//>
        <//>
    <//>`
}

// MARK: - Saved setups

function SaveSetupSheet({ onClose }) {
    const [name, setName] = useState('')
    const save = () => {
        if (name.trim()) invoke('presets.add', name.trim())
        onClose()
    }
    return html`<${Sheet} onClose=${onClose} label="Save Setup">
        <div style=${{ width: '300px', padding: '20px', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px', textAlign: 'center' }}>
            <img src="/assets/icon.png" width="56" height="56" alt="" />
            <div style=${{ fontSize: '13px', fontWeight: 700 }}>Save Setup</div>
            <div style=${{ fontSize: '11px', color: 'var(--secondary)' }}>Saves every setting, including the quality and format in the main window.</div>
            <${TextInput} label="Name" placeholder="Name" value=${name} onChange=${setName} onEnter=${save} style=${{ width: '100%' }} autoFocus />
            <div class="row" style=${{ width: '100%', gap: '8px', paddingTop: '4px' }}>
                <button class="bezel-button spacer" onClick=${onClose}>Cancel</button>
                <button class="bezel-button default spacer" onClick=${save}>Save</button>
            </div>
        </div>
    <//>`
}

function ManageSetupsSheet({ presets, onClose }) {
    const [selection, setSelection] = useState(null)
    const [renaming, setRenaming] = useState('')
    const [dragging, setDragging] = useState(null)
    useEffect(() => {
        setRenaming(presets.find((preset) => preset.id === selection)?.name ?? '')
    }, [selection])
    const rename = () => {
        if (selection && renaming.trim()) invoke('presets.rename', selection, renaming.trim())
    }
    return html`<${Sheet} onClose=${onClose} label="Saved Setups">
        <div style=${{ width: '440px', height: '360px', padding: '16px', display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <div style=${{ fontSize: '13px', fontWeight: 700 }}>Saved Setups</div>
            <div class="table-wrap" style=${{ flex: 1, height: 'auto' }} role="listbox">
                ${presets.map(
                    (preset, index) => html`<div
                        key=${preset.id}
                        role="option"
                        aria-selected=${preset.id === selection}
                        draggable="true"
                        onDragStart=${() => setDragging(index)}
                        onDragOver=${(event) => event.preventDefault()}
                        onDrop=${() => {
                            if (dragging != null && dragging !== index) invoke('presets.move', dragging, index)
                            setDragging(null)
                        }}
                        onClick=${() => setSelection(preset.id)}
                        style=${{
                            height: '24px',
                            padding: '0 8px',
                            display: 'flex',
                            alignItems: 'center',
                            background: preset.id === selection ? 'var(--accent)' : index % 2 ? 'rgba(255,255,255,0.03)' : 'transparent',
                        }}
                    >
                        ${preset.name}
                    </div>`,
                )}
            </div>
            <div class="row">
                <${TextInput}
                    class="spacer"
                    label="Name"
                    placeholder="Name"
                    value=${renaming}
                    revision=${selection}
                    onChange=${setRenaming}
                    onEnter=${rename}
                />
                <${BezelButton} disabled=${!selection || !renaming.trim()} onClick=${rename}>Rename<//>
                <${BezelButton}
                    disabled=${!selection}
                    onClick=${() => {
                        invoke('presets.delete', selection)
                        setSelection(null)
                    }}
                    >Delete<//
                >
            </div>
            <div class="row">
                <span class="secondary spacer" style=${{ fontSize: '12px' }}>Drag to reorder.</span>
                <${BezelButton} onClick=${() => invoke('presets.restore')}>Restore Built-ins<//>
                <${BezelButton} isDefault onClick=${onClose}>Done<//>
            </div>
        </div>
    <//>`
}

// MARK: - About

function AboutSettings({ state }) {
    const { tools, app, settings } = state
    return html`<${Form}>
        <${Section}>
            <${Row}>
                <div class="about-head spacer">
                    <img src="/assets/icon.png" width="64" height="64" alt="" />
                    <div style=${{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
                        <div style=${{ fontSize: '17px', fontWeight: 600, letterSpacing: '-0.02em' }}>Purrloader</div>
                        <div class="secondary">Version ${app.version}</div>
                    </div>
                </div>
            <//>
        <//>
        <${Section}
            header="Helper tools"
            footer="Purrloader keeps yt-dlp up to date by itself. Nightly gets fixes for broken sites a few days before stable."
        >
            ${['yt-dlp', 'ffmpeg', 'ffprobe', 'deno'].map(
                (tool) => html`<${Row} key=${tool} label=${tool}>
                    ${tools.transfers[tool]
                        ? html`<${Spinner} size="small" />`
                        : html`<span class="selectable" style=${{ color: tools.versions[tool] ? 'var(--secondary)' : 'var(--red)' }}>
                              ${tools.versions[tool] ?? 'Not installed'}
                          </span>`}
                <//>`,
            )}
            <${SwitchRow} label="Keep yt-dlp up to date automatically" on=${settings.autoUpdateYtdlp} onChange=${(on) => invoke('tools.setAutoUpdate', on)} />
            <${PickerRow}
                label="yt-dlp channel"
                value=${tools.channel}
                options=${[
                    { value: 'stable', title: 'Stable' },
                    { value: 'nightly', title: 'Nightly' },
                ]}
                onChange=${(channel) => invoke('tools.setChannel', channel)}
            />
            <${Row} stack>
                <div class="row">
                    <${BezelButton} disabled=${tools.isUpdating || tools.isInstalling} onClick=${() => invoke('tools.update')}>
                        ${tools.isUpdating ? 'Updating…' : 'Update Now'}
                    <//>
                    <${BezelButton} disabled=${tools.isInstalling} onClick=${() => invoke('tools.reinstall')}>Reinstall Tools<//>
                    <span class="spacer" />
                    <${BezelButton} onClick=${() => invoke('tools.showFolder')}>Show Tools Folder<//>
                </div>
                ${tools.updateLog && html`<div class="selectable lines-3" style=${{ fontSize: '11px', color: 'var(--secondary)', paddingTop: '8px' }}>${tools.updateLog}</div>`}
            <//>
        <//>
        <${Section} footer="Built on yt-dlp, FFmpeg and Deno. Sponsor segments come from the SponsorBlock community.">
            ${[
                ['Purrloader on GitHub', app.repositoryURL],
                ['License (MIT)', app.licenseURL],
                ['Third-party notices', app.noticesURL],
            ].map(
                ([title, url]) => html`<${Row} key=${url}>
                    <button class="link-button" onClick=${() => invoke('openExternal', url)}>${title}</button>
                <//>`,
            )}
        <//>
    <//>`
}
