<script setup>
import { computed, onMounted } from 'vue'
import { useData } from 'vitepress'

const props = defineProps({
  title: {
    type: String,
    default: null
  },
  description: {
    type: String,
    default: null
  },
  keywords: {
    type: String,
    default: null
  },
  image: {
    type: String,
    default: null
  },
  author: {
    type: String,
    default: 'Wolfware'
  }
})

const { frontmatter, site } = useData()

// Compute SEO values with fallbacks
const pageTitle = computed(() => props.title || frontmatter.value.title || site.value.title)
const pageDescription = computed(() => props.description || frontmatter.value.description || site.value.description)
const pageKeywords = computed(() => props.keywords || frontmatter.value.keywords || 'moonlit, build tool, release pipeline, .NET, automation')
const pageImage = computed(() => props.image || frontmatter.value.image || 'https://moonlitbuild.dev/logo.png')
const pageAuthor = computed(() => props.author || frontmatter.value.author || 'Wolfware')

// Update metadata when component mounts
onMounted(() => {
  // Update page title
  document.title = `${pageTitle.value} | Moonlit`

  // Update meta tags
  updateMetaTag('description', pageDescription.value)
  updateMetaTag('keywords', pageKeywords.value)
  updateMetaTag('author', pageAuthor.value)

  // Update Open Graph tags
  updateMetaTag('og:title', pageTitle.value, 'property')
  updateMetaTag('og:description', pageDescription.value, 'property')
  updateMetaTag('og:image', pageImage.value, 'property')

  // Update Twitter Card tags
  updateMetaTag('twitter:title', pageTitle.value)
  updateMetaTag('twitter:description', pageDescription.value)
  updateMetaTag('twitter:image', pageImage.value)
})

// Helper function to update meta tags
function updateMetaTag(name, content, attributeName = 'name') {
  if (!content) return

  let meta = document.querySelector(`meta[${attributeName}="${name}"]`)

  if (meta) {
    meta.setAttribute('content', content)
  } else {
    meta = document.createElement('meta')
    meta.setAttribute(attributeName, name)
    meta.setAttribute('content', content)
    document.head.appendChild(meta)
  }
}
</script>

<template>
  <!-- This component doesn't render anything visible -->
</template>
