export function truncate(text: string, max: number): string {
  if (text.length <= max) return text
  return text.slice(0, max - 1) + '…'
}

export function stripHtml(html: string): string {
  return html.replace(/<[^>]*>/g, '').trim()
}
