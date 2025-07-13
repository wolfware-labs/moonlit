<script setup>
import { ref, onMounted } from 'vue'
import { withBase } from 'vitepress'

const versions = ref([])
const currentVersion = ref('Current')
const isOpen = ref(false)

// This will only run on the client side when wrapped in ClientOnly
onMounted(async () => {
  try {
    // Use withBase to ensure the path is correct
    const response = await fetch(withBase('/versions.json'))
    const data = await response.json()
    versions.value = data.versions || []
    currentVersion.value = data.current || 'Current'
  } catch (error) {
    console.error('Failed to load versions:', error)
    // Provide fallback data in case of error
    versions.value = []
    currentVersion.value = 'Current'
  }
})

function toggleDropdown() {
  isOpen.value = !isOpen.value
}

function closeDropdown() {
  isOpen.value = false
}

// Close dropdown when clicking outside
onMounted(() => {
  document.addEventListener('click', (event) => {
    const dropdown = document.querySelector('.version-dropdown')
    if (dropdown && !dropdown.contains(event.target)) {
      closeDropdown()
    }
  })
})
</script>

<template>
  <div class="version-dropdown" @click.stop>
    <button class="version-selector" @click="toggleDropdown">
      {{ currentVersion }}
      <span class="arrow" :class="{ 'arrow-down': !isOpen, 'arrow-up': isOpen }"></span>
    </button>
    <ul class="version-list" v-if="isOpen">
      <li v-for="version in versions" :key="version.text">
        <a :href="version.link" @click="closeDropdown">{{ version.text }}</a>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.version-dropdown {
  position: relative;
  display: inline-block;
  margin-right: 12px;
  margin-left: 12px;
}

.version-selector {
  display: flex;
  align-items: center;
  background-color: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 4px;
  padding: 0 12px;
  height: 32px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
  cursor: pointer;
  transition: border-color 0.25s;
}

.version-selector:hover {
  border-color: var(--vp-c-brand);
}

.arrow {
  display: inline-block;
  margin-left: 6px;
  width: 0;
  height: 0;
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
}

.arrow-down {
  border-top: 5px solid var(--vp-c-text-1);
}

.arrow-up {
  border-bottom: 5px solid var(--vp-c-text-1);
}

.version-list {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 10;
  margin: 4px 0 0;
  padding: 6px 0;
  min-width: 100%;
  list-style: none;
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-divider);
  border-radius: 4px;
  box-shadow: 0 6px 12px rgba(0, 0, 0, 0.15);
}

.version-list li {
  margin: 0;
  padding: 0;
}

.version-list a {
  display: block;
  padding: 6px 12px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
  text-decoration: none;
  white-space: nowrap;
}

.version-list a:hover {
  background-color: var(--vp-c-bg-soft);
  color: var(--vp-c-brand);
}
</style>
