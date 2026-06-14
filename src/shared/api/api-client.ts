export type StandardResponse<T> = {
  ok: boolean
  data: T | null
  error: string | null
}

export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: string; status?: number }

export class ApiClient {
  private readonly baseUrl: string
  private readonly getToken: () => string | null

  constructor(baseUrl: string, getToken: () => string | null) {
    this.baseUrl = baseUrl
    this.getToken = getToken
  }

  async get<T>(path: string): Promise<ApiResult<T>> {
    return this.request<T>(path, { method: 'GET' })
  }

  async post<T>(path: string, body?: unknown): Promise<ApiResult<T>> {
    const options: RequestInit = { method: 'POST' }
    if (body instanceof FormData) {
      options.body = body
    } else if (body !== undefined) {
      options.body = JSON.stringify(body)
    }
    return this.request<T>(path, options)
  }

  async put<T>(path: string, body?: unknown): Promise<ApiResult<T>> {
    const options: RequestInit = { method: 'PUT' }
    if (body !== undefined) {
      options.body = JSON.stringify(body)
    }
    return this.request<T>(path, options)
  }

  async delete<T>(path: string): Promise<ApiResult<T>> {
    return this.request<T>(path, { method: 'DELETE' })
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<ApiResult<T>> {
    const token = this.getToken()
    const headers = new Headers(options.headers)

    if (token) headers.set('Authorization', `Bearer ${token}`)
    if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
      headers.set('Content-Type', 'application/json')
    }

    try {
      const response = await fetch(`${this.baseUrl}${path}`, { ...options, headers })
      const payload = await response.json().catch(() => null)

      if (!response.ok) {
        return { ok: false, status: response.status, error: payload?.error ?? `HTTP ${response.status}` }
      }
      if (payload?.ok === false) {
        return { ok: false, status: response.status, error: payload.error ?? 'Unknown API error' }
      }
      return { ok: true, data: payload?.data ?? payload }
    } catch (error) {
      return { ok: false, error: error instanceof Error ? error.message : 'Network error' }
    }
  }
}
