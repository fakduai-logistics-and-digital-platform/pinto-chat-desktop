<script setup lang="ts">
defineProps<{
  open: boolean
  title?: string
  maxWidth?: string
}>()

defineEmits(['close'])
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-ink/40" @click="$emit('close')" />
        <div
          :class="[
            'relative bg-surface rounded-xl p-6 w-full',
            maxWidth ?? 'max-w-md',
          ]"
        >
          <div v-if="title" class="flex items-center justify-between mb-4">
            <h3 class="text-title-lg text-ink">{{ title }}</h3>
            <button class="text-ink-subtitle hover:text-ink transition-colors" @click="$emit('close')">✕</button>
          </div>
          <slot />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
