#!/usr/bin/env bash
# Build a debug APK and install it on an ADB-connected phone by default.
set -euo pipefail
cd "$(dirname "$0")/.."

if (( $# > 1 )); then
    echo "Usage: $0 [adb-device-serial]" >&2
    exit 1
fi

export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
adb_bin="$(command -v adb || true)"
adb_bin="${adb_bin:-$ANDROID_HOME/platform-tools/adb}"
[[ -x "$adb_bin" ]] || { echo "Install Android SDK platform-tools (adb)." >&2; exit 1; }

serial="${1:-${RECETACO_ANDROID_DEVICE:-${ANDROID_SERIAL:-}}}"
if [[ -z "$serial" ]]; then
    devices=()
    while read -r device state rest; do
        [[ "$state" == device && "$device" != emulator-* ]] || continue
        qemu="$("$adb_bin" -s "$device" shell getprop ro.kernel.qemu 2>/dev/null | tr -d '\r' || true)"
        [[ "$qemu" == 1 ]] || devices+=("$device")
    done < <("$adb_bin" devices)
    if (( ${#devices[@]} != 1 )); then
        echo "Expected one authorized physical Android device; found ${#devices[@]}. Connect/unlock your phone, authorize USB debugging, or select a target with a serial argument, RECETACO_ANDROID_DEVICE, or ANDROID_SERIAL." >&2
        "$adb_bin" devices -l
        exit 1
    fi
    serial="${devices[0]}"
fi
[[ "$("$adb_bin" -s "$serial" get-state)" == device ]]

# Leave most of the machine available. Respect Linux container CPU/RAM limits
# as well as host capacity; use one JVM for Gradle and Kotlin compilation.
cpus="$(getconf _NPROCESSORS_ONLN)"
if [[ -r /sys/fs/cgroup/cpu.max ]]; then
    read -r quota period < /sys/fs/cgroup/cpu.max
    if [[ "$quota" != max ]]; then
        limit=$(( (quota + period - 1) / period ))
        (( limit >= cpus )) || cpus=$limit
    fi
fi
workers=$(( cpus / 2 ))
(( workers >= 1 )) || workers=1
(( workers <= 4 )) || workers=4
available_mb="$(awk '/MemAvailable:/ { print int($2 / 1024) }' /proc/meminfo)"
if [[ -r /sys/fs/cgroup/memory.max && -r /sys/fs/cgroup/memory.current ]]; then
    read -r memory_limit < /sys/fs/cgroup/memory.max
    read -r memory_used < /sys/fs/cgroup/memory.current
    if [[ "$memory_limit" != max ]]; then
        remaining_mb=$(( (memory_limit - memory_used) / 1024 / 1024 ))
        (( remaining_mb >= available_mb )) || available_mb=$remaining_mb
    fi
fi
if (( available_mb < 2048 )); then
    echo "At least 2 GiB available memory is required; found ${available_mb} MiB. Free memory before building." >&2
    exit 1
fi
heap_mb=$(( available_mb / 4 ))
(( heap_mb <= 2048 )) || heap_mb=2048
(( available_mb >= 8192 )) || workers=1

echo "Building for $serial: $workers workers, ${heap_mb} MiB heap, no parallel projects or persistent daemon."
bash android/gradlew -p android :app:assembleDebug \
    --no-daemon --no-parallel --max-workers="$workers" --console=plain \
    "-Dorg.gradle.jvmargs=-Xmx${heap_mb}m -XX:ActiveProcessorCount=$workers -Dfile.encoding=UTF-8" \
    -Pkotlin.compiler.execution.strategy=in-process

"$adb_bin" -s "$serial" install -r android/app/build/outputs/apk/debug/app-debug.apk
echo "Installed dev.recetaco.app on $serial."
