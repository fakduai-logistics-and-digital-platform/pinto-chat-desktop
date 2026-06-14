export const env = {
  apiBaseUrl: import.meta.env.VITE_PINTO_API_BASE_URL || 'https://api-dev.pinto-app.com',
  wsUrl: import.meta.env.VITE_PINTO_WS_URL || 'wss://api-dev.pinto-app.com',
  qrAuthWsUrl: import.meta.env.VITE_PINTO_QR_AUTH_WS_URL || deriveQrAuthWsUrl(),
  tauriPlatform: import.meta.env.TAURI_PLATFORM as string | undefined,
  tauriArch: import.meta.env.TAURI_ARCH as string | undefined,
  tauriFamily: import.meta.env.TAURI_FAMILY as string | undefined,
} as const

function deriveQrAuthWsUrl(): string {
  const base = import.meta.env.VITE_PINTO_API_BASE_URL || 'https://api-dev.pinto-app.com'
  return base.replace(/^http/, 'ws') + '/ws/auth/qr'
}
