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

TARGET="${1:-}"
if [ -n "$TARGET" ]; then
    note "Добавляю цель $TARGET"
    rustup target add "$TARGET"
    note "Сборка APK для $TARGET"
    cargo apk build --release --target "$TARGET" --manifest-path "$MANIFEST"
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
    cargo apk build --release --manifest-path "$MANIFEST"
fi

APK="$(find "$ROOT/target" -name '*.apk' -newermt '-10 minutes' | head -n 1 || true)"
if [ -n "$APK" ]; then
    note "Готово: $APK"
    printf 'Установка на устройство: adb install -r %s\n' "$APK"
else
    note "Сборка завершена, APK ищите в target/*/apk/"
fi
