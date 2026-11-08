#!/bin/zsh
# Builds a release DMG: build/Purrloader.dmg, signed and notarized when a Developer ID is set up.
#
#   PURRLOADER_SIGN_IDENTITY   "Developer ID Application: Your Name (TEAMID)" from your keychain
#   PURRLOADER_NOTARY_PROFILE  a notarytool keychain profile, created once with:
#                              xcrun notarytool store-credentials <profile> --apple-id … --team-id …
#
# Without them it still produces an ad-hoc signed DMG, which macOS warns about on other Macs.
set -euo pipefail
cd "$(dirname "$0")"

VERSION="$(<VERSION)"
APP="build/Purrloader.app"
DMG="build/Purrloader.dmg"
IDENTITY="${PURRLOADER_SIGN_IDENTITY:-}"
PROFILE="${PURRLOADER_NOTARY_PROFILE:-}"

warn() { print -P "%F{yellow}warning:%f $1" >&2; }

if [[ "$IDENTITY" != "Developer ID Application:"* ]] || ! security find-identity -v -p codesigning | grep -qF -- "$IDENTITY"; then
    warn "No Developer ID Application identity in PURRLOADER_SIGN_IDENTITY; building an ad-hoc signed DMG."
    warn "It won't be notarized, so other Macs will refuse to open it until the user allows it in System Settings."
    IDENTITY=""
fi
if [[ -n "$IDENTITY" && -z "$PROFILE" ]]; then
    warn "PURRLOADER_NOTARY_PROFILE isn't set; the DMG will be signed but not notarized."
fi

# build.sh signs ad-hoc (or with a development identity); release signing happens below.
PURRLOADER_SIGN_IDENTITY="" ./build.sh

notarize() {
    local file="$1"
    echo "Notarizing $file…"
    xcrun notarytool submit "$file" --keychain-profile "$PROFILE" --wait
    xcrun stapler staple "${2:-$file}"
}

if [[ -n "$IDENTITY" ]]; then
    echo "Signing $APP with $IDENTITY…"
    codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP"
    codesign --verify --strict --verbose=2 "$APP"
    if [[ -n "$PROFILE" ]]; then
        # notarytool takes a zip, but the ticket is stapled to the app itself.
        ZIP="build/Purrloader-notarize.zip"
        ditto -c -k --keepParent "$APP" "$ZIP"
        notarize "$ZIP" "$APP"
        rm -f "$ZIP"
    fi
fi

dmg/make-dmg.sh

if [[ -n "$IDENTITY" ]]; then
    codesign --force --timestamp --sign "$IDENTITY" "$DMG"
    if [[ -n "$PROFILE" ]]; then notarize "$DMG"; fi
    spctl --assess --type open --context context:primary-signature --verbose "$DMG" || true
fi

echo "Built $DMG (Purrloader $VERSION)"
