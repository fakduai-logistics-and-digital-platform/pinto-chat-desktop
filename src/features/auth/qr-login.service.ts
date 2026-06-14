/**
 * QR Login Service
 *
 * Desktop QR login is NOT autonomous auth — it binds a waiting web session to an
 * already-logged-in mobile user. The backend issues web/admin tokens after validating
 * the mobile user's Bearer token via POST /api/auth/qr/confirm.
 *
 * Flow:
 *   1. Desktop opens WS → receives {type:'init', session_id, expires_in}
 *   2. Desktop renders QR containing {type_mode:'scan_login', session_id}
 *   3. Mobile (logged-in user) scans QR, sends POST /api/auth/qr/confirm
 *      with its own Bearer token and body {session_id}
 *   4. Backend validates mobile token + session, issues web/admin tokens
 *   5. Backend pushes {type:'login_success', access_token, refresh_token} to WS
 *   6. Desktop stores tokens via same path as email/password login
 */

import { env } from '@/app/env'

export type QrSessionStatus = 'connecting' | 'pending' | 'scanned' | 'confirmed' | 'expired' | 'error'

export type QrSession = {
  sessionId: string
  payload: string
  status: QrSessionStatus
  expiresAt: number
}

/** Tokens received from backend via WS after mobile confirms the QR session. */
export type QrLoginTokens = {
  access_token: string
  refresh_token?: string
}

/**
 * User data that may be included in login_success WS message.
 * Used as fallback if the profile endpoint fails.
 */
export type QrWsUser = {
  user_id?: string
  id?: string
  username?: string
  name?: string
  profile_image?: string
}

type QrWsMessage =
  | { type: 'init'; session_id?: string; expires_in?: number; data?: { session_id?: string; expires_in?: number } }
  | { type: 'login_success'; access_token?: string; accessToken?: string; refresh_token?: string; refreshToken?: string; user?: QrWsUser }
  | { type: 'expired' }
  | { type: 'error'; message?: string; data?: { message?: string } }

let ws: WebSocket | null = null
let countdownTimer: ReturnType<typeof setInterval> | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let intentionalClose = false
let connectionId = 0

/**
 * Opens a WebSocket to the QR auth endpoint. The backend holds this session open
 * until either the mobile user confirms (login_success) or the session expires.
 * On expiry, automatically reconnects with a fresh session.
 *
 * Uses a connectionId to prevent stale callbacks from a previous connection
 * when switching tabs, refreshing, or reconnecting quickly.
 */
export function connectQrWs(
  onSessionReady: (sessionId: string, expiresIn: number) => void,
  onConfirmed: (tokens: QrLoginTokens, user?: QrWsUser) => void,
  onError: (message: string) => void,
  onStatusChange: (status: QrSessionStatus) => void,
): () => void {
  disconnectQrWs()
  intentionalClose = false
  onStatusChange('connecting')

  const id = ++connectionId
  const wsUrl = env.qrAuthWsUrl

  let openedWs: WebSocket
  try {
    openedWs = new WebSocket(wsUrl)
  } catch {
    onStatusChange('error')
    onError('ไม่สามารถเชื่อมต่อ WebSocket ได้')
    return () => disconnectQrWs()
  }
  ws = openedWs

  openedWs.onopen = () => {
    if (id !== connectionId) return
    onStatusChange('pending')
  }

  openedWs.onmessage = (event) => {
    if (id !== connectionId) return
    let msg: QrWsMessage
    try {
      msg = JSON.parse(event.data)
    } catch {
      return
    }

    switch (msg.type) {
      case 'init': {
        const sessionId = msg.session_id ?? msg.data?.session_id
        const expiresIn = msg.expires_in ?? msg.data?.expires_in ?? 60
        if (sessionId) {
          onSessionReady(sessionId, expiresIn)
          onStatusChange('pending')
        }
        break
      }
      case 'login_success': {
        const accessToken = msg.access_token ?? msg.accessToken
        const refreshToken = msg.refresh_token ?? msg.refreshToken
        if (accessToken) {
          onStatusChange('confirmed')
          onConfirmed({ access_token: accessToken, refresh_token: refreshToken }, msg.user)
        }
        break
      }
      case 'expired': {
        onStatusChange('expired')
        scheduleReconnect(onSessionReady, onConfirmed, onError, onStatusChange)
        break
      }
      case 'error': {
        const errMsg = msg.message ?? msg.data?.message ?? 'เกิดข้อผิดพลาด'
        onStatusChange('error')
        onError(errMsg)
        break
      }
    }
  }

  openedWs.onerror = () => {
    if (id !== connectionId) return
    onStatusChange('error')
    onError('WebSocket connection error')
  }

  openedWs.onclose = () => {
    if (id !== connectionId) return
    if (!intentionalClose) {
      scheduleReconnect(onSessionReady, onConfirmed, onError, onStatusChange)
    }
  }

  return () => disconnectQrWs()
}

function scheduleReconnect(
  onSessionReady: (sessionId: string, expiresIn: number) => void,
  onConfirmed: (tokens: QrLoginTokens, user?: QrWsUser) => void,
  onError: (message: string) => void,
  onStatusChange: (status: QrSessionStatus) => void,
) {
  if (reconnectTimer) clearTimeout(reconnectTimer)
  reconnectTimer = setTimeout(() => {
    connectQrWs(onSessionReady, onConfirmed, onError, onStatusChange)
  }, 3000)
}

export function disconnectQrWs() {
  intentionalClose = true
  connectionId++
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  stopCountdown()
  if (ws) {
    ws.onclose = null
    ws.close()
    ws = null
  }
}

export function startCountdown(expiresIn: number, onTick: (remaining: number) => void, onExpired: () => void): void {
  stopCountdown()
  let remaining = expiresIn
  onTick(remaining)
  countdownTimer = setInterval(() => {
    remaining--
    if (remaining <= 0) {
      stopCountdown()
      onExpired()
    } else {
      onTick(remaining)
    }
  }, 1000)
}

export function stopCountdown() {
  if (countdownTimer) {
    clearInterval(countdownTimer)
    countdownTimer = null
  }
}

/**
 * Build the QR code payload. Mobile scans this to identify which waiting
 * web session to bind to. The payload itself does NOT carry credentials.
 */
export function buildQrPayload(sessionId: string): string {
  return JSON.stringify({
    type_mode: 'scan_login',
    session_id: sessionId,
  })
}
