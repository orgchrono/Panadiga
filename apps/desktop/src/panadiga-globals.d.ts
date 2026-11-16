/// <reference types="vite/client" />

/**
 * Injected statically at Vite build time (`false` on desktop `.exe`/`.deb`/macOS bundles,
 * `true` on `vite build --mode android` for `panadiga-android-arm64.apk`).
 */
declare const __PANADIGA_ANDROID_APK__: boolean;
