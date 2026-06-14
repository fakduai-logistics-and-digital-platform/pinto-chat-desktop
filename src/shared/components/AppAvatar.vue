<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RiUser3Fill } from '@remixicon/vue'

const props = defineProps<{
  src?: string | null
  name?: string
  size?: 'sm' | 'md' | 'lg'
  online?: boolean
}>()

const imageFailed = ref(false)

const hasImage = computed(() => Boolean(props.src) && !imageFailed.value)

watch(
  () => props.src,
  () => {
    imageFailed.value = false
  },
)
</script>

<template>
  <div class="relative inline-flex shrink-0">
    <img
      v-if="hasImage"
      :src="src!"
      :alt="name"
      :class="[
        'rounded-full object-cover bg-white',
        { 'w-8 h-8': size === 'sm' || !size, 'w-10 h-10': size === 'md', 'w-14 h-14': size === 'lg' },
      ]"
      @error="imageFailed = true"
    />
    <div
      v-else
      :class="[
        'rounded-full flex items-center justify-center bg-[#F1F1F1] text-[#8F8F8F] overflow-hidden',
        { 'w-8 h-8': size === 'sm' || !size, 'w-10 h-10': size === 'md', 'w-14 h-14': size === 'lg' },
      ]"
    >
      <RiUser3Fill
        :class="[
          { 'h-4 w-4': size === 'sm' || !size, 'h-5 w-5': size === 'md', 'h-7 w-7': size === 'lg' },
        ]"
      />
    </div>
  </div>
</template>
