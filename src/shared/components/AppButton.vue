<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  label?: string
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline'
  size?: 'sm' | 'md' | 'lg'
  loading?: boolean
  disabled?: boolean
  icon?: boolean
}>(), {
  variant: 'primary',
  size: 'sm',
  loading: false,
  disabled: false,
  icon: false,
})

defineEmits(['click'])

const classes = computed(() => [
  'app-button',
  `app-button--${props.variant}`,
  `app-button--${props.size}`,
  {
    'app-button--icon': props.icon,
    'app-button--disabled': props.disabled || props.loading,
  },
])
</script>

<template>
  <button
    :class="classes"
    :disabled="disabled || loading"
    @click="$emit('click')"
  >
    <span v-if="loading" class="app-button__spinner" />
    <span class="app-button__content">
      <slot>{{ label }}</slot>
    </span>
  </button>
</template>

<style scoped>
.app-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 1px solid transparent;
  font-family: 'IBM Plex Sans Thai', system-ui, sans-serif;
  font-weight: 600;
  line-height: 1;
  border-radius: 12px;
  transition: background-color 150ms ease, border-color 150ms ease, color 150ms ease, box-shadow 150ms ease, transform 150ms ease, opacity 150ms ease;
  user-select: none;
  white-space: nowrap;
  cursor: pointer;
}

.app-button__content {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: inherit;
  opacity: 1;
  visibility: visible;
}

.app-button--sm {
  height: 36px;
  padding: 0 12px;
  font-size: 0.875rem;
  border-radius: 10px;
}

.app-button--md {
  height: 44px;
  padding: 0 16px;
  font-size: 1rem;
  border-radius: 12px;
}

.app-button--lg {
  height: 52px;
  padding: 0 20px;
  font-size: 1.125rem;
  border-radius: 16px;
}

.app-button--icon {
  width: 40px;
  padding: 0;
}

.app-button--primary {
  background-color: #2ECC71;
  color: #FFFFFF;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.08);
}

.app-button--primary:hover {
  background-color: #27AE60;
  color: #FFFFFF;
}

.app-button--primary:active {
  background-color: #1E8A4B;
  color: #FFFFFF;
  box-shadow: none;
  transform: scale(0.98);
}

.app-button--secondary {
  background-color: #EAFBF1;
  color: #1E8A4B;
}

.app-button--secondary:hover {
  background-color: #D4F7E3;
  color: #1E8A4B;
}

.app-button--ghost {
  background-color: transparent;
  color: #696373;
}

.app-button--ghost:hover {
  background-color: #F9FAFB;
  color: #433D4D;
}

.app-button--danger {
  background-color: #FF4C51;
  color: #FFFFFF;
}

.app-button--danger:hover {
  background-color: #FF7074;
  color: #FFFFFF;
}

.app-button--outline {
  background-color: #FFFFFF;
  color: #433D4D;
  border-color: #D4D1D8;
}

.app-button--outline:hover {
  background-color: #F9FAFB;
  color: #433D4D;
}

.app-button--disabled,
.app-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

.app-button__spinner {
  width: 16px;
  height: 16px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 9999px;
  animation: app-button-spin 1s linear infinite;
}

@keyframes app-button-spin {
  to { transform: rotate(360deg); }
}
</style>
