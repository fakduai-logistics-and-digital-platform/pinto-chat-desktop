/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
  export default component
}

interface ImportMetaEnv {
  readonly VITE_PINTO_API_BASE_URL: string
  readonly VITE_PINTO_WS_URL: string
  readonly VITE_PINTO_QR_AUTH_WS_URL: string
  readonly TAURI_PLATFORM: string | undefined
  readonly TAURI_ARCH: string | undefined
  readonly TAURI_FAMILY: string | undefined
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
