#!/bin/zsh
# Packs build/Purrloader.app into build/Purrloader.dmg with the drag-to-Applications window.
# Usage: dmg/make-dmg.sh   (run ./build.sh first)
set -euo pipefail
cd "$(dirname "$0")/.."

APP="build/Purrloader.app"
DMG="build/Purrloader.dmg"
NAME="Purrloader"
[[ -d "$APP" ]] || { echo "No $APP; run ./build.sh first." >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'hdiutil detach "/Volumes/$NAME" -quiet 2>/dev/null || true; rm -rf "$WORK"' EXIT

# Background at 1x and 2x, combined into one TIFF so Retina screens get the sharp one.
swift dmg/background.swift "$WORK/bg.png" 1
swift dmg/background.swift "$WORK/bg@2x.png" 2
tiffutil -cathidpicheck "$WORK/bg.png" "$WORK/bg@2x.png" -out "$WORK/background.tiff" 2>/dev/null

STAGE="$WORK/stage"
mkdir -p "$STAGE/.background"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cp "$WORK/background.tiff" "$STAGE/.background/background.tiff"
cp "$APP/Contents/Resources/Purrloader.icns" "$STAGE/.VolumeIcon.icns"

# A writable image first, so Finder can save the window layout into it.
hdiutil detach "/Volumes/$NAME" -quiet 2>/dev/null || true
hdiutil create -quiet -srcfolder "$STAGE" -volname "$NAME" -fs HFS+ -format UDRW -ov "$WORK/rw.dmg"
hdiutil attach -quiet -readwrite -noverify -noautoopen "$WORK/rw.dmg"
xcrun SetFile -a C "/Volumes/$NAME" 2>/dev/null || true # use .VolumeIcon.icns

# Window: 640x400 content; the icons sit where background.swift leaves room for them.
osascript <<EOF
tell application "Finder"
    tell disk "$NAME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set bounds of container window to {200, 120, 840, 548}
        set opts to icon view options of container window
        set arrangement of opts to not arranged
        set icon size of opts to 112
        set text size of opts to 12
        set background picture of opts to file ".background:background.tiff"
        set position of item "$NAME.app" of container window to {170, 205}
        set position of item "Applications" of container window to {470, 205}
        close
        open
        update without registering applications
        delay 2
        close
    end tell
end tell
EOF

sync
hdiutil detach -quiet "/Volumes/$NAME"
rm -f "$DMG"
hdiutil convert -quiet "$WORK/rw.dmg" -format ULMO -o "$DMG"
echo "Built $DMG"
