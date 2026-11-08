#!/usr/bin/env python3
"""Package a native release binary. Run on the target OS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
PACKAGING = Path(__file__).resolve().parent
VERSION = (ROOT / 'VERSION').read_text().strip()


def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def download_appimagetool(arch, work):
    # Pin the asset and its publisher digest from one release response. Fail closed
    # if the rolling release changes while downloading.
    request = urllib.request.Request('https://api.github.com/repos/AppImage/appimagetool/releases/latest', headers={'User-Agent': 'Purrloader-build'})
    with urllib.request.urlopen(request, timeout=30) as response:
        release = json.load(response)
    name = f'appimagetool-{arch}.AppImage'
    asset = next(a for a in release['assets'] if a['name'] == name)
    expected = asset.get('digest', '')
    if not expected.startswith('sha256:'):
        raise RuntimeError('AppImage publisher did not provide a SHA-256 digest')
    path = work / name
    with urllib.request.urlopen(asset['browser_download_url'], timeout=120) as response, path.open('wb') as output:
        shutil.copyfileobj(response, output)
    actual = 'sha256:' + hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise RuntimeError('AppImage tool checksum mismatch')
    path.chmod(0o755)
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--arch', choices=['x64', 'arm64'], required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--licenses', type=Path, required=True)
    parser.add_argument('--format', choices=['all', 'archive', 'deb', 'appimage', 'nsis'], default='all')
    parser.add_argument('--output', type=Path, default=ROOT / 'desktop/dist')
    args = parser.parse_args()
    binary = args.binary.resolve()
    licenses = args.licenses.resolve()
    if not binary.is_file() or not licenses.is_file():
        parser.error('Build the release binary and generate dependency licenses first')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    os_name = platform.system()
    # Protect against silently labelling a host binary with the other architecture.
    if os_name == 'Windows':
        with binary.open('rb') as source:
            source.seek(0x3c)
            offset = int.from_bytes(source.read(4), 'little')
            source.seek(offset + 4)
            machine = int.from_bytes(source.read(2), 'little')
        expected = 0x8664 if args.arch == 'x64' else 0xaa64
    elif os_name == 'Linux':
        with binary.open('rb') as source:
            header = source.read(20)
        if header[:4] != b'\x7fELF':
            parser.error('Expected a Linux ELF executable')
        machine = int.from_bytes(header[18:20], 'little')
        expected = 62 if args.arch == 'x64' else 183
    else:
        parser.error('Build Windows packages on Windows and Linux packages on Linux')
    if machine != expected:
        parser.error('Binary architecture does not match --arch')
    if os_name == 'Windows':
        if args.format not in ('all', 'nsis'):
            parser.error('Windows uses --format nsis')
        nsis = shutil.which('makensis') or str(Path(os.environ.get('ProgramFiles(x86)', r'C:\Program Files (x86)')) / 'NSIS/makensis.exe')
        numeric = '.'.join((VERSION.split('.') + ['0'] * 4)[:4])
        definitions = dict(OUTPUT=output / f'Purrloader-Setup-{args.arch}.exe', VERSION=VERSION, NUMVERSION=numeric, BINARY=binary, ICON=ROOT / 'desktop/assets/icon.ico', LICENSE=ROOT / 'LICENSE', NOTICES=ROOT / 'THIRD_PARTY_NOTICES.md', DEPENDENCY_LICENSES=licenses)
        run(nsis, *(f'/D{k}={v}' for k, v in definitions.items()), PACKAGING / 'windows.nsi')
        return
    if args.format == 'nsis':
        parser.error('NSIS packages are built on Windows')
    with tempfile.TemporaryDirectory(prefix='purrloader-package-') as temp:
        work = Path(temp)
        appdir = work / 'Purrloader.AppDir'
        copy(binary, appdir / 'usr/bin/purrloader')
        (appdir / 'usr/bin/purrloader').chmod(0o755)
        copy(PACKAGING / 'purrloader.desktop', appdir / 'purrloader.desktop')
        copy(ROOT / 'desktop/assets/icon.png', appdir / 'purrloader.png')
        copy(ROOT / 'desktop/assets/icon.png', appdir / 'usr/share/icons/hicolor/1024x1024/apps/purrloader.png')
        copy(PACKAGING / 'purrloader.desktop', appdir / 'usr/share/applications/purrloader.desktop')
        for path in [ROOT / 'LICENSE', ROOT / 'THIRD_PARTY_NOTICES.md']:
            copy(path, appdir / 'usr/share/doc/purrloader' / path.name)
        copy(licenses, appdir / 'usr/share/doc/purrloader/THIRD_PARTY_LICENSES.html')
        launcher = appdir / 'AppRun'
        launcher.write_text('#!/bin/sh\nHERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$HERE/usr/bin/purrloader" "$@"\n')
        launcher.chmod(0o755)
        if args.format in ('all', 'archive'):
            portable = work / 'Purrloader'
            shutil.copytree(appdir, portable)
            shutil.copy2(portable / 'AppRun', portable / 'purrloader')
            executables = {'Purrloader/AppRun', 'Purrloader/purrloader', 'Purrloader/usr/bin/purrloader'}

            def normalize(info):
                # Set permissions explicitly, so the archive is right even when built on Windows.
                info.uid = info.gid = 0
                info.uname = info.gname = 'root'
                info.mode = 0o755 if info.isdir() or info.name in executables else 0o644
                return info

            with tarfile.open(output / f'Purrloader-linux-{args.arch}.tar.gz', 'w:gz') as archive:
                archive.add(portable, arcname='Purrloader', filter=normalize)
        if args.format in ('all', 'deb'):
            deb = work / 'deb'
            shutil.copytree(appdir / 'usr', deb / 'usr')
            control = deb / 'DEBIAN/control'
            control.parent.mkdir()
            arch = 'amd64' if args.arch == 'x64' else 'arm64'
            size = sum(p.stat().st_size for p in deb.rglob('*') if p.is_file()) // 1024
            control.write_text(f'Package: purrloader\nVersion: {VERSION}\nArchitecture: {arch}\nMaintainer: BLCNYY <blcnyy@users.noreply.github.com>\nSection: video\nPriority: optional\nInstalled-Size: {size}\nDepends: libc6 (>= 2.35), libgcc-s1, libgl1, libxkbcommon0, libxkbcommon-x11-0, libx11-6, libxcb1, libwayland-client0, liblzma5, xdg-utils\nDescription: The quiet way to save videos\n Save videos and audio from thousands of sites. Paste a link, press Download.\n')
            run('dpkg-deb', '--root-owner-group', '--build', deb, output / f'purrloader_{arch}.deb')
        if args.format in ('all', 'appimage'):
            arch = 'x86_64' if args.arch == 'x64' else 'aarch64'
            tool = download_appimagetool(arch, work)
            # Extract-and-run works without FUSE in CI. The generated app needs the
            # distribution's graphics libraries, listed alongside the .deb.
            env = dict(os.environ, ARCH=arch, APPIMAGE_EXTRACT_AND_RUN='1')
            filename = 'x86_64' if args.arch == 'x64' else 'arm64'
            run(tool, '--no-appstream', appdir, output / f'Purrloader-{filename}.AppImage', env=env)


if __name__ == '__main__':
    main()
