// The Mac app's custom controls (Components/Controls.swift) and stand-ins for the AppKit ones it uses:
// switches, pop-up buttons, steppers, grouped forms, menus, popovers and sheets.

import { Icon } from './icons.js'
import {
    cx,
    html,
    useCallback,
    useEffect,
    useLayoutEffect,
    useRef,
    useState,
} from './lib.js'

// MARK: - Pill segment

/** Capsule segmented control with a sliding white pill. Each segment is a real button. */
export function PillSegment({ value, options, onChange, label, height = 32 }) {
    const refs = useRef({})
    const [indicator, setIndicator] = useState(null)
    useLayoutEffect(() => {
        const button = refs.current[value]
        if (button) setIndicator({ left: button.offsetLeft, width: button.offsetWidth })
    }, [value, options.map((o) => o[1]).join('|')])
    return html`<div class="pill" role="group" aria-label=${label}>
        ${indicator && html`<div class="pill-indicator" style=${{ left: indicator.left + 'px', width: indicator.width + 'px' }} />`}
        ${options.map(
            ([option, title]) =>
                html`<button
                    key=${option}
                    ref=${(el) => (refs.current[option] = el)}
                    class=${cx('pill-option', option === value && 'selected')}
                    style=${{ height: height - 4 + 'px' }}
                    aria-pressed=${option === value}
                    onClick=${() => onChange(option)}
                >
                    ${title}
                </button>`,
        )}
    </div>`
}

// MARK: - Chips and buttons

export function ChipButton({ text, icon, chevron = false, onClick, title, label, open, buttonRef, value }) {
    return html`<button
        ref=${buttonRef}
        class=${cx('chip', open && 'open')}
        title=${title}
        aria-label=${label}
        aria-valuetext=${value}
        aria-haspopup=${chevron ? 'menu' : undefined}
        aria-expanded=${chevron ? Boolean(open) : undefined}
        onClick=${onClick}
    >
        ${icon && html`<${Icon} name=${icon} size=${12} weight="medium" />`}
        <span>${text}</span>
        ${chevron && html`<${Icon} name="chevronDown" size=${9} weight="bold" style=${{ color: 'var(--tertiary)' }} />`}
    </button>`
}

/** A picker shown as a capsule chip that opens a menu. */
export function ChipMenu({ name, text, icon, items, title }) {
    const ref = useRef(null)
    const [open, setOpen] = useState(false)
    return html`<${ChipButton}
        buttonRef=${ref}
        text=${text}
        icon=${icon}
        chevron
        open=${open}
        title=${title}
        label=${name}
        value=${text}
        onClick=${() => {
            setOpen(true)
            openMenu({ items, anchor: ref.current, onClose: () => setOpen(false) })
        }}
    />`
}

export function CircleButton({ icon, size = 32, filled = true, label, onClick, disabled, iconSize }) {
    return html`<button
        class=${cx('circle-button', !filled && 'plain')}
        style=${{ width: size + 'px', height: size + 'px' }}
        title=${label}
        aria-label=${label}
        disabled=${disabled}
        onClick=${onClick}
    >
        <${Icon} name=${icon} size=${iconSize ?? Math.round(size * 0.42)} weight="semibold" />
    </button>`
}

export function PrimaryButton({ children, onClick, disabled, title }) {
    return html`<button class="primary-button" title=${title} disabled=${disabled} onClick=${onClick}>${children}</button>`
}

/** A quieter capsule for actions next to the primary one, like an error's fix-it button. */
export function SecondaryButton({ children, onClick, buttonRef, height = 28 }) {
    return html`<button ref=${buttonRef} class="secondary-button" style=${{ height: height + 'px' }} onClick=${onClick}>
        ${children}
    </button>`
}

export function BezelButton({ children, onClick, disabled, isDefault, title }) {
    return html`<button class=${cx('bezel-button', isDefault && 'default')} title=${title} disabled=${disabled} onClick=${onClick}>
        ${children}
    </button>`
}

// MARK: - Indicators

/** Thin capsule progress bar; null shows an indeterminate sweep. */
export function SlimProgress({ fraction, tint }) {
    return html`<div class="slim-progress" role="progressbar" aria-valuenow=${fraction != null ? Math.round(fraction * 100) : undefined}>
        ${fraction != null
            ? html`<div class="fill" style=${{ width: `${Math.max(0, Math.min(1, fraction)) * 100}%`, background: tint }} />`
            : html`<div class="sweep" style=${{ background: tint }} />`}
    </div>`
}

export function Tag({ text, color }) {
    const background = color === '#fff' ? 'rgba(255,255,255,0.15)' : `color-mix(in srgb, ${color} 15%, transparent)`
    return html`<span class="tag" style=${{ color, background }}>${text}</span>`
}

/** The Mac's spinning activity indicator. */
export function Spinner({ size = 'small' }) {
    const px = { mini: 11, small: 16, regular: 28 }[size] ?? 16
    const spokes = Array.from({ length: 12 }, (_, i) => {
        const angle = i * 30
        return html`<rect
            x="11"
            y="2"
            width="2"
            height="6"
            rx="1"
            fill="currentColor"
            opacity=${(0.25 + (i / 12) * 0.75).toFixed(2)}
            transform=${`rotate(${angle} 12 12)`}
        />`
    })
    return html`<svg class="spinner" width=${px} height=${px} viewBox="0 0 24 24" role="progressbar" aria-label="Loading">
        ${spokes}
    </svg>`
}

export function Thumbnail({ url, width }) {
    const [failed, setFailed] = useState(false)
    useEffect(() => setFailed(false), [url])
    const radius = Math.max(6, width * 0.08)
    return html`<div class="thumbnail" style=${{ width: width + 'px', height: (width * 9) / 16 + 'px', borderRadius: radius + 'px' }} aria-hidden="true">
        <${Icon} name="playFill" size=${Math.max(8, width * 0.16)} />
        ${url && !failed && html`<img src=${url} referrerpolicy="no-referrer" loading="lazy" onError=${() => setFailed(true)} />`}
    </div>`
}

/** Shrinks the middle of a long title, the way the Mac truncates download names. */
export function MiddleText({ text, class: className }) {
    const tailLength = Math.min(16, Math.floor(text.length / 3))
    if (text.length < 24) return html`<div class=${cx('ellipsis', className)}>${text}</div>`
    return html`<div class=${cx('middle', className)} title=${text}>
        <span class="head">${text.slice(0, text.length - tailLength)}</span><span class="tail">${text.slice(-tailLength)}</span>
    </div>`
}

// MARK: - Form controls

export function Switch({ on, onChange, label, disabled }) {
    return html`<button
        class=${cx('switch', on && 'on')}
        role="switch"
        aria-checked=${Boolean(on)}
        aria-label=${label}
        disabled=${disabled}
        onClick=${() => onChange(!on)}
    />`
}

export function Checkbox({ on, onChange, label }) {
    return html`<button class=${cx('checkbox', on && 'on')} role="checkbox" aria-checked=${Boolean(on)} onClick=${() => onChange(!on)}>
        <span class="box">${on && html`<${Icon} name="checkmark" size=${10} weight="bold" />`}</span>
        <span>${label}</span>
    </button>`
}

/** A Mac pop-up button: shows the chosen title and opens a menu with a checkmark on it. */
export function PopUpButton({ value, options, onChange, plain = false, label, disabled, minWidth }) {
    const ref = useRef(null)
    const current = options.find((option) => !option.separator && option.value === value)
    return html`<button
        ref=${ref}
        class=${cx('popup', plain && 'plain')}
        aria-label=${label}
        aria-haspopup="menu"
        disabled=${disabled}
        style=${{ minWidth: minWidth ? minWidth + 'px' : undefined }}
        onClick=${() =>
            openMenu({
                anchor: ref.current,
                align: plain ? 'right' : 'left',
                minWidth: ref.current.offsetWidth,
                items: options.map((option) =>
                    option.separator
                        ? { separator: true }
                        : { label: option.title, checked: option.value === value, action: () => onChange(option.value) },
                ),
            })}
    >
        <span>${current?.title ?? ''}</span>
        <${Icon} name="chevronUpDown" size=${11} weight="semibold" style=${{ color: 'var(--secondary)' }} />
    </button>`
}

export function Stepper({ value, min, max, step = 1, onChange, label }) {
    return html`<span class="stepper" role="group" aria-label=${label}>
        <button aria-label="Decrease" disabled=${value <= min} onClick=${() => onChange(Math.max(min, value - step))}>
            <${Icon} name="minus" size=${10} weight="bold" />
        </button>
        <button aria-label="Increase" disabled=${value >= max} onClick=${() => onChange(Math.min(max, value + step))}>
            <${Icon} name="plus" size=${10} weight="bold" />
        </button>
    </span>`
}

export function Segmented({ value, options, onChange, label }) {
    return html`<div class="segmented" role="group" aria-label=${label}>
        ${options.map(
            ([option, title]) =>
                html`<button key=${option} class=${cx(option === value && 'selected')} aria-pressed=${option === value} onClick=${() => onChange(option)}>
                    ${title}
                </button>`,
        )}
    </div>`
}

/** A text field that keeps what's being typed while the main process catches up. It takes the stored value when
 * it isn't being edited, or when `revision` changes (a reset, a saved setup). */
export function TextInput({
    value,
    revision,
    onChange,
    onEnter,
    placeholder,
    class: className,
    multiline = false,
    rows = 2,
    label,
    width,
    mono = false,
    inputRef,
    style,
    autoFocus,
}) {
    const [local, setLocal] = useState(value ?? '')
    const focused = useRef(false)
    useEffect(() => {
        if (!focused.current) setLocal(value ?? '')
    }, [value])
    useEffect(() => setLocal(value ?? ''), [revision])
    const props = {
        class: cx('text-input', mono && 'mono', className),
        value: local,
        placeholder,
        'aria-label': label ?? placeholder,
        spellcheck: false,
        autoFocus,
        style: { width: width ? width + 'px' : undefined, ...style },
        ref: inputRef,
        onInput: (event) => {
            setLocal(event.target.value)
            onChange?.(event.target.value)
        },
        onFocus: () => (focused.current = true),
        onBlur: () => {
            focused.current = false
        },
        onKeyDown: (event) => {
            if (event.key === 'Enter' && !multiline) onEnter?.(event)
        },
    }
    return multiline ? html`<textarea rows=${rows} ...${props} />` : html`<input type="text" ...${props} />`
}

// MARK: - Grouped forms

export function Form({ children }) {
    return html`<div class="form">${children}</div>`
}

export function Section({ header, footer, children }) {
    return html`<section class="form-section">
        ${header && html`<div class="form-header" role="heading" aria-level="2">${header}</div>`}
        <div class="form-box">${children}</div>
        ${footer && html`<div class="form-footer">${footer}</div>`}
    </section>`
}

export function Row({ label, sub, children, stack = false, class: className }) {
    return html`<div class=${cx('form-row', stack && 'stack', className)}>
        ${label != null && html`<div class="label">${label}${sub && html`<span class="sub">${sub}</span>`}</div>`}
        ${children != null && (stack ? children : html`<div class="value">${children}</div>`)}
    </div>`
}

export function SwitchRow({ label, sub, on, onChange, disabled }) {
    return html`<${Row} label=${label} sub=${sub}>
        <${Switch} on=${on} onChange=${onChange} label=${label} disabled=${disabled} />
    <//>`
}

export function PickerRow({ label, value, options, onChange }) {
    return html`<${Row} label=${label}>
        <${PopUpButton} plain value=${value} options=${options} onChange=${onChange} label=${label} />
    <//>`
}

export function enumOptions(titles) {
    return Object.entries(titles).map(([value, title]) => ({ value, title }))
}

// MARK: - Menus

let showMenu = null

/** Opens a menu below `anchor` (an element or a {x, y} point). Items: {label, checked, disabled, shortcut,
 * action} | {separator: true} | {section: 'Title'}. */
export function openMenu(menu) {
    showMenu?.(menu)
}

/** Draws menus like the Mac's: one host per window. */
export function MenuHost() {
    const [menu, setMenu] = useState(null)
    const [active, setActive] = useState(-1)
    const [position, setPosition] = useState(null)
    const box = useRef(null)
    showMenu = (next) => {
        setActive(-1)
        setPosition(null)
        setMenu(next)
    }
    const close = useCallback(() => {
        setMenu((current) => {
            current?.onClose?.()
            return null
        })
    }, [])

    useLayoutEffect(() => {
        if (!menu || !box.current) return
        const rect = box.current.getBoundingClientRect()
        let x
        let y
        if (menu.anchor instanceof Element) {
            const anchor = menu.anchor.getBoundingClientRect()
            x = menu.align === 'right' ? anchor.right - rect.width : anchor.left
            y = anchor.bottom + 4
            if (y + rect.height > window.innerHeight - 8 && anchor.top - rect.height - 4 > 8) y = anchor.top - rect.height - 4
        } else {
            x = menu.anchor.x
            y = menu.anchor.y
        }
        x = Math.max(8, Math.min(x, window.innerWidth - rect.width - 8))
        y = Math.max(8, Math.min(y, window.innerHeight - rect.height - 8))
        setPosition({ x, y })
        box.current.focus()
    }, [menu])

    if (!menu) return null
    const items = menu.items
    const selectable = (index) => items[index] && !items[index].separator && !items[index].section && !items[index].disabled
    const choose = (item) => {
        close()
        // After the menu is gone, so a dialog the action opens isn't covered by it.
        setTimeout(() => item.action?.(), 0)
    }
    const move = (direction) => {
        let index = active
        for (let step = 0; step < items.length; step++) {
            index = (index + direction + items.length) % items.length
            if (selectable(index)) return setActive(index)
        }
    }
    const onKeyDown = (event) => {
        event.stopPropagation()
        if (event.key === 'ArrowDown') move(1)
        else if (event.key === 'ArrowUp') move(-1)
        else if (event.key === 'Escape' || event.key === 'Tab') close()
        else if ((event.key === 'Enter' || event.key === ' ') && selectable(active)) choose(items[active])
        else return
        event.preventDefault()
    }

    return html`<div class="layer" data-menu-layer onMouseDown=${(event) => event.target === event.currentTarget && close()} onContextMenu=${(e) => {
        e.preventDefault()
        close()
    }}>
        <div
            ref=${box}
            class="menu"
            role="menu"
            tabindex="-1"
            style=${{
                left: (position?.x ?? 0) + 'px',
                top: (position?.y ?? 0) + 'px',
                minWidth: menu.minWidth ? menu.minWidth + 'px' : undefined,
                visibility: position ? 'visible' : 'hidden',
            }}
            onKeyDown=${onKeyDown}
            onMouseLeave=${() => setActive(-1)}
        >
            ${items.map((item, index) => {
                if (item.separator) return html`<div key=${index} class="menu-separator" role="separator" />`
                if (item.section) return html`<div key=${index} class="menu-section">${item.section}</div>`
                return html`<button
                    key=${index}
                    class=${cx('menu-item', index === active && 'active')}
                    role=${item.checked != null ? 'menuitemradio' : 'menuitem'}
                    aria-checked=${item.checked != null ? Boolean(item.checked) : undefined}
                    disabled=${item.disabled}
                    onMouseEnter=${() => !item.disabled && setActive(index)}
                    onClick=${() => choose(item)}
                >
                    <span class="check">${item.checked && html`<${Icon} name="checkmark" size=${11} weight="bold" />`}</span>
                    <span class="spacer">${item.label}</span>
                    ${item.shortcut && html`<span class="shortcut">${item.shortcut}</span>`}
                </button>`
            })}
        </div>
    </div>`
}

// MARK: - Popovers and sheets

/** A panel hanging from its anchor with an arrow, closed by clicking elsewhere or pressing Escape. */
export function Popover({ anchor, onClose, children, width }) {
    const box = useRef(null)
    const [position, setPosition] = useState(null)
    const place = useCallback(() => {
        if (!anchor || !box.current) return
        const a = anchor.getBoundingClientRect()
        const panelWidth = box.current.offsetWidth
        const center = a.left + a.width / 2
        const x = Math.max(8, Math.min(center - panelWidth / 2, window.innerWidth - panelWidth - 8))
        setPosition({ x, y: a.bottom + 9, arrow: center - x })
    }, [anchor])
    useLayoutEffect(() => {
        place()
        window.addEventListener('resize', place)
        return () => window.removeEventListener('resize', place)
    }, [place])
    useEffect(() => {
        const handle = (event) => {
            if (box.current?.contains(event.target) || anchor?.contains(event.target)) return
            if (event.target.closest?.('[data-menu-layer], .sheet-backdrop')) return
            onClose()
        }
        const key = (event) => event.key === 'Escape' && onClose()
        document.addEventListener('mousedown', handle, true)
        document.addEventListener('keydown', key)
        return () => {
            document.removeEventListener('mousedown', handle, true)
            document.removeEventListener('keydown', key)
        }
    }, [anchor, onClose])
    return html`<div
        ref=${box}
        class="popover"
        role="dialog"
        style=${{
            left: (position?.x ?? 0) + 'px',
            top: (position?.y ?? 0) + 'px',
            width: width ? width + 'px' : undefined,
            visibility: position ? 'visible' : 'hidden',
        }}
    >
        ${position && html`<div class="popover-arrow" style=${{ left: position.arrow + 'px' }} />`}
        <div style=${{ maxHeight: `calc(100vh - ${(position?.y ?? 0) + 8}px)`, overflowY: 'auto', borderRadius: '14px' }}>
            ${children}
        </div>
    </div>`
}

/** A sheet over the window, like SwiftUI's .sheet. */
export function Sheet({ onClose, children, label }) {
    useEffect(() => {
        if (!onClose) return
        const key = (event) => {
            if (event.key === 'Escape' && !document.querySelector('[data-menu-layer]')) onClose()
        }
        document.addEventListener('keydown', key)
        return () => document.removeEventListener('keydown', key)
    }, [onClose])
    return html`<div class="sheet-backdrop">
        <div class="sheet" role="dialog" aria-modal="true" aria-label=${label}>${children}</div>
    </div>`
}
