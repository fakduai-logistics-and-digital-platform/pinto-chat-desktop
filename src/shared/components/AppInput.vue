<script setup lang="ts">
defineProps<{
  modelValue: string
  placeholder?: string
  type?: string
  disabled?: boolean
  error?: string
}>()

defineEmits(['update:modelValue', 'enter'])
</script>

<template>
  <div class="w-full">
    <input
      :type="type ?? 'text'"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :class="[
        'w-full px-3 py-2.5 text-body-sm bg-surface text-content placeholder-ink-hint rounded-sm transition-all duration-150',
        error
          ? 'border-2 border-error focus:border-error'
          : 'border border-input-border focus:border-pinto focus:shadow-[0_0_0_3px_rgba(46,204,113,0.12)]',
        disabled ? 'opacity-50 cursor-not-allowed bg-surface-muted' : '',
      ]"
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      @keydown.enter="$emit('enter')"
    />
    <p v-if="error" class="mt-1 text-caption text-error">{{ error }}</p>
  </div>
</template>
