// https://vitepress.dev/guide/custom-theme
import { h, onMounted } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import './style.css'
import SEOMetadata from './components/SEOMetadata.vue'

export default {
  extends: DefaultTheme,
  Layout: () => {
    // Include SEOMetadata component in the layout
    return h('div', [
      h(SEOMetadata),
      h(DefaultTheme.Layout, null, {
        // https://vitepress.dev/guide/extending-default-theme#layout-slots
      })
    ])
  },
  enhanceApp({ app, router, siteData }) {
    // Register SEOMetadata component globally
    app.component('SEOMetadata', SEOMetadata)
  },
  setup() {
    // Add JSON-LD structured data for SEO
    onMounted(() => {
      const jsonLd = {
        '@context': 'https://schema.org',
        '@type': 'SoftwareApplication',
        'name': 'Moonlit',
        'description': 'A powerful build and release pipeline tool built on .NET',
        'applicationCategory': 'DeveloperApplication',
        'operatingSystem': 'Windows, Linux, macOS',
        'offers': {
          '@type': 'Offer',
          'price': '0',
          'priceCurrency': 'USD'
        },
        'author': {
          '@type': 'Organization',
          'name': 'Wolfware',
          'url': 'https://wolfware.dev'
        }
      };

      const script = document.createElement('script');
      script.setAttribute('type', 'application/ld+json');
      script.textContent = JSON.stringify(jsonLd);
      document.head.appendChild(script);
    });
  }
} satisfies Theme
