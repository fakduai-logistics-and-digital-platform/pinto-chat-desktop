<script setup lang="ts">
defineProps<{
  icon?: string
  size?: 'sm' | 'md'
  variant?: 'ghost' | 'secondary'
}>()

defineEmits(['click'])
</script>

<template>
  <button
    :class="[
      'inline-flex items-center justify-center rounded-md transition-colors focus:outline-none',
      {
        'text-ink-secondary hover:bg-surface-muted': variant === 'ghost' || !variant,
        'text-ink-secondary bg-surface-muted hover:bg-hairline': variant === 'secondary',
        'w-8 h-8 text-xs': size === 'sm',
        'w-9 h-9 text-sm': size === 'md' || !size,
      },
    ]"
    @click="$emit('click')"
  >
    <slot />
  </button>
</template>
