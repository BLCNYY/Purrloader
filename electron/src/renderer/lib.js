// Preact with htm templates, the state the main process sends, and small helpers every window uses.

import { Fragment, h, render } from '/vendor/preact.js'
import {
    useCallback,
    useEffect,
    useLayoutEffect,
    useMemo,
    useReducer,
    useRef,
    useState,
} from '/vendor/hooks.js'
import htm from '/vendor/htm.js'

export const html = htm.bind(h)
export { Fragment, h, render, useCallback, useEffect, useLayoutEffect, useMemo, useReducer, useRef, useState }

export const api = window.purrloader
export const platform = api.platform
export const isMac = platform === 'darwin'

/** Sends an action to the main process. Failures are logged rather than thrown into event handlers. */
export function invoke(action, ...args) {
    return api.invoke(action, ...args).catch((error) => {
        console.error(action, error)
        return null
    })
}

// MARK: - State

let state = {}
const listeners = new Set()
let frame = 0

export async function loadState() {
    state = await api.invoke('getState')
    api.onState((patch) => {
        state = { ...state, ...patch }
        // Several patches in one frame cause one render.
        if (!frame) {
            frame = requestAnimationFrame(() => {
                frame = 0
                for (const listener of listeners) listener()
            })
        }
    })
}

/** The whole app state; the component re-renders when it changes. */
export function useAppState() {
    const [, tick] = useReducer((n) => n + 1, 0)
    useEffect(() => {
        listeners.add(tick)
        return () => listeners.delete(tick)
    }, [])
    return state
}

export function getState() {
    return state
}

// MARK: - Helpers

export function cx(...names) {
    return names.filter(Boolean).join(' ')
}

/** The modifier key for shortcuts: Ctrl on Windows and Linux, ⌘ on a Mac. */
export function isShortcut(event) {
    return isMac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey
}

export function shortcutLabel(keys) {
    if (isMac) return keys.replace('Ctrl+', '⌘').replace('Alt+', '⌥').replace('Shift+', '⇧')
    return keys
}

export function useClickOutside(ref, onOutside, active = true) {
    useEffect(() => {
        if (!active) return
        const handle = (event) => {
            if (ref.current && !ref.current.contains(event.target)) onOutside(event)
        }
        const blur = () => onOutside()
        // Capture, so a click that opens something else still closes this first.
        document.addEventListener('mousedown', handle, true)
        window.addEventListener('blur', blur)
        return () => {
            document.removeEventListener('mousedown', handle, true)
            window.removeEventListener('blur', blur)
        }
    }, [active, onOutside])
}

export function useKey(key, handler, active = true) {
    useEffect(() => {
        if (!active) return
        const listener = (event) => {
            if (event.key === key) handler(event)
        }
        document.addEventListener('keydown', listener)
        return () => document.removeEventListener('keydown', listener)
    }, [key, handler, active])
}
