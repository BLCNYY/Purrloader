// The only bridge between a window and the main process: state comes in, named actions go out.
const { contextBridge, ipcRenderer, webUtils } = require('electron')

contextBridge.exposeInMainWorld('purrloader', {
    platform: process.platform,
    invoke: (action, ...args) => ipcRenderer.invoke('purrloader:invoke', action, ...args),
    onState(callback) {
        const listener = (_event, patch) => callback(patch)
        ipcRenderer.on('purrloader:state', listener)
        return () => ipcRenderer.removeListener('purrloader:state', listener)
    },
    /** The path of a file dropped on the window. */
    pathForFile: (file) => webUtils.getPathForFile(file),
})
