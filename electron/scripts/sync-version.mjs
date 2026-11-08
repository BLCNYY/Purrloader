// The repository's VERSION file is the one place the version lives (the Mac build reads it too). This writes it
// into package.json as semver, so "1.0" becomes "1.0.0".
import fs from 'node:fs'

const version = fs.readFileSync(new URL('../../VERSION', import.meta.url), 'utf8').trim()
const parts = version.split('.')
while (parts.length < 3) parts.push('0')
const file = new URL('../package.json', import.meta.url)
const manifest = JSON.parse(fs.readFileSync(file, 'utf8'))
if (manifest.version !== parts.join('.')) {
    manifest.version = parts.join('.')
    fs.writeFileSync(file, JSON.stringify(manifest, null, 2) + '\n')
}
console.log(`Purrloader ${manifest.version}`)
