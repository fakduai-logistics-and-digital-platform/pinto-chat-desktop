<script setup lang="ts">
defineProps<{
  title?: string
  description?: string
  icon?: string
}>()
</script>

<template>
  <div class="flex flex-col items-center justify-center py-12 text-center">
    <div class="w-16 h-16 rounded-full bg-pinto-50 flex items-center justify-center text-2xl mb-4">
      {{ icon ?? '💬' }}
    </div>
    <h3 class="text-headline-md text-ink mb-1">{{ title ?? 'Nothing here' }}</h3>
    <p class="text-body-sm text-ink-secondary max-w-xs">{{ description ?? 'No content available' }}</p>
    <slot />
  </div>
</template>
