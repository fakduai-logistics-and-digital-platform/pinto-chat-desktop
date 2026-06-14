<script setup lang="ts">
defineProps<{ date: string }>()
</script>

<template>
  <div class="flex items-center justify-center my-4">
    <div class="px-3 py-1 bg-surface-muted border border-hairline rounded-pill text-caption text-ink-subtitle font-medium">
      {{ date }}
    </div>
  </div>
</template>
