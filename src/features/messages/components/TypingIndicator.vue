<script setup lang="ts">
defineProps<{ name: string }>()
</script>

<template>
  <div class="flex items-center gap-2 px-4 py-1">
    <div class="flex gap-1">
      <span class="w-1.5 h-1.5 bg-pinto rounded-full animate-bounce" style="animation-delay: 0ms" />
      <span class="w-1.5 h-1.5 bg-pinto rounded-full animate-bounce" style="animation-delay: 150ms" />
      <span class="w-1.5 h-1.5 bg-pinto rounded-full animate-bounce" style="animation-delay: 300ms" />
    </div>
    <span class="text-caption text-ink-subtitle">{{ name }} กำลังพิมพ์...</span>
  </div>
</template>
