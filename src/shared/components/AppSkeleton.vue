<script setup lang="ts">
defineProps<{ count?: number }>()
</script>

<template>
  <div class="space-y-3 p-4">
    <div v-for="i in (count ?? 5)" :key="i" class="flex items-center space-x-3">
      <div class="w-10 h-10 bg-hairline rounded-full shrink-0 animate-pulse" />
      <div class="flex-1 space-y-2">
        <div class="h-3 bg-hairline rounded-xs w-1/3 animate-pulse" />
        <div class="h-3 bg-hairline rounded-xs w-2/3 animate-pulse" />
      </div>
    </div>
  </div>
</template>
