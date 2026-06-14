<script setup lang="ts">
import { ref, onUnmounted, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/features/auth/auth.store'
import QRCode from 'qrcode'
import {
  connectQrWs,
  disconnectQrWs,
  startCountdown,
  stopCountdown,
  buildQrPayload,
  type QrSessionStatus,
  type QrLoginTokens,
  type QrWsUser,
} from '@/features/auth/qr-login.service'
import AuthLayout from '@/layouts/AuthLayout.vue'
import AppButton from '@/shared/components/AppButton.vue'
import AppInput from '@/shared/components/AppInput.vue'

const router = useRouter()
const auth = useAuthStore()

type LoginMode = 'qr' | 'email'
const mode = ref<LoginMode>('qr')

const email = ref('')
const password = ref('')
const isLoading = ref(false)
const error = ref<string | null>(null)
const emailError = ref('')
const passwordError = ref('')

const qrStatus = ref<QrSessionStatus>('connecting')
const qrError = ref<string | null>(null)
const qrDataUrl = ref('')
const qrGenerating = ref(false)
const qrSessionId = ref<string | null>(null)
const qrCountdown = ref(0)
const formattedCountdown = computed(() => {
  const m = Math.floor(qrCountdown.value / 60)
  const s = qrCountdown.value % 60
  return `${m}:${s.toString().padStart(2, '0')}`
})

function switchMode(m: LoginMode) {
  disconnectQrWs()
  stopCountdown()
  qrSessionId.value = null
  qrStatus.value = 'connecting'
  qrError.value = null
  qrDataUrl.value = ''
  qrCountdown.value = 0
  error.value = null
  emailError.value = ''
  passwordError.value = ''
  mode.value = m
  if (m === 'qr') initQr()
}

async function generateQr(sessionId: string) {
  qrGenerating.value = true
  try {
    const payload = buildQrPayload(sessionId)
    qrDataUrl.value = await QRCode.toDataURL(payload, {
      width: 220,
      margin: 2,
      color: { dark: '#433D4D', light: '#FFFFFF' },
      errorCorrectionLevel: 'M',
    })
  } catch {
    qrError.value = 'ไม่สามารถสร้าง QR Code ได้'
  } finally {
    qrGenerating.value = false
  }
}

function initQr() {
  qrStatus.value = 'connecting'
  qrError.value = null
  qrDataUrl.value = ''
  qrSessionId.value = null
  qrCountdown.value = 0

  connectQrWs(
    (sessionId, expiresIn) => {
      qrSessionId.value = sessionId
      generateQr(sessionId)
      startCountdown(
        Math.min(expiresIn, 60),
        (remaining) => { qrCountdown.value = remaining },
        () => { qrStatus.value = 'expired' },
      )
    },
    async (tokens: QrLoginTokens, wsUser?: QrWsUser) => {
      stopCountdown()
      const ok = await auth.loginWithQrTokens(tokens.access_token, tokens.refresh_token, wsUser)
      if (ok) router.push('/')
      else qrError.value = auth.error ?? 'Failed to load profile'
    },
    (errMsg) => {
      qrError.value = errMsg
      qrStatus.value = 'error'
    },
    (status) => { qrStatus.value = status },
  )
}

function refreshQr() {
  disconnectQrWs()
  stopCountdown()
  initQr()
}

function validateForm(): boolean {
  let valid = true
  emailError.value = ''
  passwordError.value = ''
  if (!email.value.trim()) {
    emailError.value = 'Email or username is required'
    valid = false
  }
  if (!password.value) {
    passwordError.value = 'Password is required'
    valid = false
  }
  return valid
}

async function handleEmailLogin() {
  if (!validateForm()) return
  isLoading.value = true
  error.value = null
  const success = await auth.login({ email: email.value, password: password.value })
  isLoading.value = false
  if (success) router.push('/')
  else error.value = auth.error ?? 'Login failed'
}

if (mode.value === 'qr') initQr()
watch(mode, (m) => { if (m === 'qr' && !qrSessionId.value) initQr() })

onUnmounted(() => {
  disconnectQrWs()
  stopCountdown()
})
</script>

<template>
  <AuthLayout>
    <div class="w-full max-w-[600px]">
      <div :class="[
        'mx-auto w-full max-w-sm bg-surface px-6 py-7',
        mode === 'email' ? 'rounded-xl border border-divider shadow-card' : '',
      ]">
        <div v-if="mode === 'email'" class="text-center mb-6">
          <div class="inline-flex h-16 w-16 items-center justify-center rounded-full bg-pinto-50 mb-4 ring-1 ring-pinto-200/70">
            <span class="text-3xl">🍊</span>
          </div>
          <h1 class="text-headline-md text-content">
            <span class="text-pinto font-bold">Pinto</span>
            <span class="font-bold"> Chat</span>
          </h1>
          <p class="text-body-sm text-ink-secondary mt-1">ลงชื่อเข้าใช้บัญชีของคุณ</p>
        </div>

      <!-- Tabs -->
        <div v-if="mode === 'email'" class="flex mb-6 rounded-sm border border-hairline bg-surface-muted p-1">
        <button
          class="flex-1 py-2.5 text-label-sm rounded-sm text-ink-secondary transition-all duration-150 hover:text-pinto"
          @click="switchMode('qr')"
        >
          สแกน QR Code
        </button>
        <button
          :class="[
            'flex-1 py-2.5 text-label-sm rounded-sm transition-all duration-150',
            mode === 'email'
              ? 'bg-surface text-pinto font-semibold shadow-card'
              : 'text-ink-secondary hover:text-pinto',
          ]"
          @click="switchMode('email')"
        >
          อีเมล / รหัสผ่าน
        </button>
      </div>

      <!-- Error banner -->
      <div v-if="error" class="mb-4 p-3 bg-error-container border border-error/30 rounded-sm text-body-sm text-error-strong">
        {{ error }}
      </div>

      <!-- QR Code Mode -->
      <div v-if="mode === 'qr'" class="flex flex-col items-center text-center">
        <div class="mb-10">
          <h2 class="text-title-lg font-semibold text-ink">สแกน QR Code ด้วยแอป Pinto</h2>
          <p class="mt-1 text-body-sm text-ink-secondary">เข้าสู่ระบบอย่างปลอดภัยบนอุปกรณ์นี้</p>
        </div>

        <div class="relative mb-4">
          <div
            :class="[
              'flex h-48 w-48 items-center justify-center rounded-2xl bg-surface p-4 shadow-[0_20px_60px_rgba(46,204,113,0.10)] ring-1',
              qrStatus === 'expired' || qrStatus === 'error' || qrError
                ? 'ring-error/20'
                : 'ring-[#2ECC71]',
            ]"
          >
            <img
              v-if="qrDataUrl && qrStatus !== 'expired' && qrStatus !== 'error' && !qrError"
              :src="qrDataUrl"
              alt="QR Code"
              class="h-full w-full object-contain"
            />
            <div v-else-if="qrStatus === 'expired' || qrStatus === 'error' || qrError" class="flex h-full w-full flex-col items-center justify-center gap-3 rounded-lg bg-error-container px-4">
              <div class="flex h-11 w-11 items-center justify-center rounded-full bg-surface text-error-strong ring-1 ring-error/20">
                <svg class="h-6 w-6" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="M12 8v4" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
                  <path d="M12 16h.01" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
                  <path d="M10.3 4.3 2.8 17.4A2 2 0 0 0 4.5 20h15a2 2 0 0 0 1.7-2.6L13.7 4.3a2 2 0 0 0-3.4 0Z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
                </svg>
              </div>
              <div>
                <p class="text-label-sm text-ink">{{ qrError ? 'เชื่อมต่อไม่สำเร็จ' : 'QR Code หมดอายุแล้ว' }}</p>
                <p class="mt-1 text-caption text-ink-secondary">กดสร้างใหม่เพื่อรับรหัสล่าสุด</p>
              </div>
            </div>
            <div v-else class="flex h-full flex-col items-center justify-center gap-2">
              <div class="h-6 w-6 animate-spin rounded-full border-2 border-pinto border-t-transparent" />
              <span class="text-caption text-ink-hint">
                {{ qrStatus === 'connecting' ? 'กำลังเชื่อมต่อ...' : 'กำลังสร้าง QR Code...' }}
              </span>
            </div>
          </div>

          <!-- Scanned overlay -->
          <div v-if="qrStatus === 'scanned'" class="absolute inset-0 flex flex-col items-center justify-center rounded-xl border-2 border-pinto/20 bg-surface/95">
            <div class="mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-pinto-50 text-pinto">
              <svg class="h-6 w-6" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <rect x="7" y="3" width="10" height="18" rx="2" stroke="currentColor" stroke-width="2" />
                <path d="M11 18h2" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              </svg>
            </div>
            <div class="text-label-md text-ink">สแกนแล้ว</div>
            <div class="text-body-sm text-ink-secondary">กรุณายืนยันบนโทรศัพท์</div>
          </div>

          <!-- Confirmed overlay -->
          <div v-if="qrStatus === 'confirmed'" class="absolute inset-0 flex flex-col items-center justify-center rounded-xl border-2 border-pinto/30 bg-success-container/95">
            <div class="mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-pinto">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none"><path d="M5 13l4 4L19 7" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </div>
            <div class="text-label-md text-pinto-700">ยืนยันแล้ว</div>
          </div>
          </div>

          <button
            v-if="qrStatus === 'expired' || qrStatus === 'error' || qrError"
            type="button"
            class="mb-12 mt-2 w-48 rounded-sm bg-pinto-50 px-4 py-3 text-label-md font-semibold text-pinto transition duration-200 hover:bg-pinto-100 active:scale-[0.98] focus:outline-none focus:ring-2 focus:ring-pinto/30"
            @click="refreshQr"
          >
            สร้าง QR Code ใหม่
          </button>

          <!-- Countdown -->
          <div v-if="qrCountdown > 0 && qrStatus !== 'scanned' && qrStatus !== 'confirmed' && qrStatus !== 'expired' && qrStatus !== 'error' && !qrError" class="mb-16 flex items-center justify-center gap-2 text-label-md">
            <svg class="h-5 w-5 text-pinto" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path d="M12 8v5l3 2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              <path d="M7 4.8A8 8 0 1 1 4.8 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              <path d="M4 4v4h4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span class="text-ink">หมดอายุ</span>
            <span class="font-bold text-pinto">{{ formattedCountdown }}</span>
          </div>

          <p class="text-body-sm text-ink-secondary">เปิดแอป Pinto ในมือถือของคุณ</p>
      </div>

      <!-- Email / Password Mode -->
      <form v-else @submit.prevent="handleEmailLogin" class="space-y-4">
        <div>
          <label class="block text-label-sm text-ink mb-2">อีเมล หรือ ชื่อผู้ใช้</label>
          <AppInput v-model="email" placeholder="you@example.com" :disabled="isLoading" :error="emailError" />
        </div>
        <div>
          <label class="block text-label-sm text-ink mb-2">รหัสผ่าน</label>
          <AppInput v-model="password" type="password" placeholder="••••••••" :disabled="isLoading" :error="passwordError" @enter="handleEmailLogin" />
        </div>
        <AppButton :loading="isLoading" variant="primary" class="w-full" @click="handleEmailLogin">
          เข้าสู่ระบบ
        </AppButton>
      </form>
      </div>
    </div>
  </AuthLayout>
</template>
