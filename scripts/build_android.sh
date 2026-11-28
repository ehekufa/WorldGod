#!/usr/bin/env bash
# Локальная сборка APK без Gradle и без Java-кода проекта.
#
# Требуется:
#   * Rust (rustup) с целью aarch64-linux-android;
#   * Android SDK (переменная ANDROID_HOME) с build-tools и platforms;
#   * Android NDK (переменная ANDROID_NDK_HOME);
#   * cargo-apk:  cargo install cargo-apk --version "^0.10"
#
# Пример:
#   scripts/build_android.sh                 # все ABI из Cargo.toml
#   scripts/build_android.sh aarch64-linux-android
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$ROOT/crates/worldgod_app/Cargo.toml"

note() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31mОшибка:\033[0m %s\n' "$*" >&2; exit 1; }

command -v cargo >/dev/null || fail "не найден cargo — установите Rust: https://rustup.rs"
command -v cargo-apk >/dev/null || fail "не найден cargo-apk — выполните: cargo install cargo-apk --version \"^0.10\""
[ -n "${ANDROID_HOME:-}" ] || fail "не задан ANDROID_HOME (путь к Android SDK)"
[ -n "${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}" ] || fail "не задан ANDROID_NDK_HOME (путь к Android NDK)"

# cargo-apk требует ключ для `--release`. В CI он одноразовый, локально
# создаём dev-ключ в target/ (в git он не попадает) и переиспользуем.
ensure_release_key() {
    if [ -n "${CARGO_APK_RELEASE_KEYSTORE:-}" ]; then
        return 0
    fi
    local keystore="$ROOT/target/worldgod-dev.keystore"
    if [ ! -f "$keystore" ]; then
        command -v keytool >/dev/null || fail "не найден keytool (JDK) — он нужен для ключа подписи"
        note "Создаю dev-ключ подписи: $keystore"
        mkdir -p "$(dirname "$keystore")"
        keytool -genkeypair -noprompt \
            -keystore "$keystore" -storetype PKCS12 \
            -storepass worldgod -keypass worldgod \
            -alias worldgod -keyalg RSA -keysize 2048 -validity 10000 \
            -dname "CN=WorldGod dev, O=WorldGod, C=RU" >/dev/null
    fi
    export CARGO_APK_RELEASE_KEYSTORE="$keystore"
    export CARGO_APK_RELEASE_KEYSTORE_PASSWORD=worldgod
}

ensure_release_key

TARGET="${1:-}"
if [ -n "$TARGET" ]; then
    note "Добавляю цель $TARGET"
    rustup target add "$TARGET"
    note "Сборка APK для $TARGET"
    cargo apk build --release --target "$TARGET" --manifest-path "$MANIFEST" || true
else
    note "Сборка APK для всех ABI из [package.metadata.android].build_targets"
    while IFS= read -r line; do
        case "$line" in
            *linux-android*|*androideabi*)
                target="$(printf '%s' "$line" | tr -d ' ",' )"
                note "rustup target add $target"
                rustup target add "$target"
                ;;
        esac
    done < <(sed -n '/^build_targets/,/]/p' "$MANIFEST")
    cargo apk build --release --manifest-path "$MANIFEST" || true
fi

# cargo-apk 0.10 умеет собрать и выровнять APK, но на подписи иногда падает,
# оставляя готовый неподписанный файл. Поэтому после сборки проверяем подпись и
# при необходимости подписываем сами тем же ключом через apksigner из SDK.
sign_apk() {
    local apk="$1"
    local signer
    signer="$(find "${ANDROID_HOME}/build-tools" -maxdepth 2 -name apksigner | sort -V | tail -n 1)"
    [ -n "$signer" ] || fail "не найден apksigner в ${ANDROID_HOME}/build-tools"

    if "$signer" verify "$apk" >/dev/null 2>&1; then
        return 0
    fi
    note "APK не подписан — подписываю через apksigner"
    "$signer" sign --ks "$CARGO_APK_RELEASE_KEYSTORE" \
        --ks-pass "pass:$CARGO_APK_RELEASE_KEYSTORE_PASSWORD" "$apk"
    "$signer" verify --verbose "$apk" >/dev/null
    note "Подпись проверена"
}

APK="$(find "$ROOT/target" -name '*.apk' -newermt '-10 minutes' | head -n 1 || true)"
if [ -n "$APK" ]; then
    sign_apk "$APK"
    note "Готово: $APK"
    printf 'Установка на устройство: adb install -r %s\n' "$APK"
else
    note "Сборка завершена, APK ищите в target/*/apk/"
fi
