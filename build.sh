#!/bin/zsh
# Builds Purrloader.app into ./build. Usage: ./build.sh [--run | --install]
set -euo pipefail
cd "$(dirname "$0")"

VERSION="$(<VERSION)"
# A universal binary, so the same app runs on Apple Silicon and Intel Macs.
BUILD=(swift build -c release --arch arm64 --arch x86_64)
"${BUILD[@]}"
BIN="$("${BUILD[@]}" --show-bin-path)/Purrloader"
APP="build/Purrloader.app"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/Purrloader"
cp Resources/Info.plist "$APP/Contents/Info.plist"
# VERSION is the one place the version lives; About and the update check read it from the bundle.
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APP/Contents/Info.plist"
# A fresh build number every build, so macOS's icon cache notices a changed icon.
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $(date +%Y%m%d%H%M%S)" "$APP/Contents/Info.plist"
# Compile the Icon Composer icon: Assets.car (Liquid Glass) plus a Purrloader.icns fallback.
xcrun actool Resources/Purrloader.icon --compile "$APP/Contents/Resources" --platform macosx \
    --minimum-deployment-target 15.0 --target-device mac --app-icon Purrloader \
    --output-partial-info-plist build/icon-info.plist >/dev/null
# Set PURRLOADER_SIGN_IDENTITY to a signing identity in your keychain (e.g. "Apple Development: …") to keep
# macOS privacy permissions such as Downloads folder access across rebuilds. Without one the build is
# ad-hoc signed, which macOS treats as a new app every time. Release signing lives in release.sh.
IDENTITY="${PURRLOADER_SIGN_IDENTITY:-}"
if [[ -z "$IDENTITY" ]] || ! security find-identity -v -p codesigning | grep -qF -- "$IDENTITY"; then IDENTITY="-"; fi
codesign --force --sign "$IDENTITY" "$APP" 2>/dev/null

echo "Built $APP ($VERSION)"

case "${1:-}" in
    --run)
        pkill -x Purrloader 2>/dev/null || true
        sleep 1
        open "$APP"
        ;;
    --install)
        # Quit gracefully so queued downloads are saved, then replace the installed copy.
        # If downloads are running, Purrloader asks before quitting; don't wait on that forever.
        if pgrep -x Purrloader >/dev/null; then
            osascript -e 'with timeout of 15 seconds' -e 'tell application id "com.blcnyy.purrloader" to quit' \
                -e 'end timeout' >/dev/null 2>&1 || true
            for _ in {1..10}; do pgrep -x Purrloader >/dev/null || break; sleep 1; done
            if pgrep -x Purrloader >/dev/null; then
                echo "Purrloader is still open (downloads in progress?). Quit it, then run this again." >&2
                exit 1
            fi
        fi
        rm -rf /Applications/Purrloader.app
        cp -R "$APP" /Applications/Purrloader.app
        echo "Installed /Applications/Purrloader.app"
        # Tell Launch Services about the new copy so Finder and the Dock pick up its icon.
        /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
            -f /Applications/Purrloader.app
        open /Applications/Purrloader.app
        ;;
esac
