import { withMermaid } from "vitepress-plugin-mermaid"
import * as fs from 'fs'
import * as path from 'path'

// https://vitepress.dev/reference/site-config
export default withMermaid({
  title: "Moonlit",
  description: "A powerful build and release pipeline tool",
  lang: 'en-US',
  lastUpdated: true,

  // SEO optimizations
  head: [
    ['meta', { name: 'author', content: 'Wolfware' }],
    ['meta', { name: 'keywords', content: 'moonlit, build tool, release pipeline, automation, CI/CD, DevOps' }],
    ['meta', { name: 'robots', content: 'index, follow' }],

    // Favicon
    ['link', { rel: 'icon', type: 'image/png', href: '/logo.png' }],

    // Open Graph / Facebook
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'Moonlit - Modern Release Pipeline' }],
    ['meta', { property: 'og:description', content: 'A powerful build and release pipeline tool' }],
    ['meta', { property: 'og:image', content: 'https://moonlitbuild.dev/logo_portrait.png' }],

    // Twitter
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:title', content: 'Moonlit - Modern Release Pipeline' }],
    ['meta', { name: 'twitter:description', content: 'A powerful build and release pipeline tool' }],
    ['meta', { name: 'twitter:image', content: 'https://moonlitbuild.dev/logo_portrait.png' }],
  ],
  // Sitemap configuration
  sitemap: {
    hostname: 'https://moonlitbuild.dev/'
  },

  // Performance optimizations
  cleanUrls: true, // Remove .html extensions from URLs

  // Generate dynamic canonical URLs and update Open Graph URL for each page
  transformHead({ pageData }) {
    // Get the page path, handling index.md files and clean URLs
    let pagePath = '';

    if (pageData.relativePath !== 'index.md') {
      // For non-homepage files
      pagePath = pageData.relativePath
        .replace(/\.md$/, '')         // Remove .md extension
        .replace(/\/index$/, '/');    // Handle nested index.md files
    }

    // Construct the full canonical URL
    const canonicalUrl = new URL(pagePath, 'https://moonlitbuild.dev/').href;

    return [
      ['link', { rel: 'canonical', href: canonicalUrl }],
      ['meta', { property: 'og:url', content: canonicalUrl }]
    ];
  },

  // Copy versions.json to the output directory during build
  buildEnd: async (siteConfig) => {
    const srcPath = path.resolve(process.cwd(), 'versions.json')
    const destPath = path.resolve(siteConfig.outDir, 'versions.json')

    if (fs.existsSync(srcPath)) {
      fs.copyFileSync(srcPath, destPath)
      console.log('Copied versions.json to output directory')
    } else {
      console.warn('versions.json not found in source directory')
    }
  },

  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/' },
      { text: 'Reference', link: '/reference/' },
      { text: 'Plugins', link: '/plugins/' }
    ],

    // Algolia DocSearch Configuration
    search: {
      provider: 'algolia',
      options: {
        appId: 'OVI0FI9ADY',
        apiKey: '31cfa2cbf48b3494a5a147fd5fdadef8',
        indexName: 'Moonlit Docs',
        placeholder: 'Search documentation',
        translations: {
          button: {
            buttonText: 'Search'
          }
        }
      }
    },

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/' },
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Quick Start', link: '/guide/quick-start' }
          ]
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'How Moonlit Works', link: '/guide/concepts/how-it-works' },
            { text: 'Plugins System', link: '/guide/concepts/plugins' },
            { text: 'Stages and Steps', link: '/guide/concepts/stages-steps' },
            { text: 'Configuration', link: '/guide/concepts/configuration' }
          ]
        },
        {
          text: 'Advanced Usage',
          items: [
            { text: 'Creating Custom Plugins', link: '/guide/advanced/custom-plugins' },
            { text: 'Dependency Injection', link: '/guide/advanced/dependency-injection' },
            { text: 'Middleware Pipeline', link: '/guide/advanced/middleware' },
            { text: 'Documentation Versioning', link: '/guide/advanced/versioning' }
          ]
        }
      ],
      '/reference/': [
        {
          text: 'CLI',
          items: [
            { text: 'Command Line Interface', link: '/reference/cli' },
            { text: 'Configuration File', link: '/reference/config-file' }
          ]
        },
        {
          text: 'API',
          items: [
            { text: 'Core API', link: '/reference/core-api' },
            { text: 'Plugin Development', link: '/reference/plugin-development' },
            { text: 'Plugin System Architecture', link: '/reference/plugin-system' }
          ]
        },
        {
          text: 'Troubleshooting',
          items: [
            { text: 'Error Handling', link: '/reference/error-handling' }
          ]
        }
      ],
      '/plugins/': [
        {
          text: 'Official Plugins',
          items: [
            { text: 'Overview', link: '/plugins/' },
            { text: 'Git Plugin', link: '/plugins/git' },
            { text: 'GitHub Plugin', link: '/plugins/github' },
            { text: 'Semantic Release Plugin', link: '/plugins/semantic-release' },
            { text: 'Slack Plugin', link: '/plugins/slack' },
            { text: 'NuGet Plugin', link: '/plugins/nuget' },
            { text: 'Docker Plugin', link: '/plugins/docker' },
            { text: 'NPM Plugin', link: '/plugins/npm' }
          ]
        },
        {
          text: 'Examples',
          items: [
            { text: 'NuGet Release Pipeline', link: '/plugins/examples/nuget-release' },
            { text: 'Docker Deployment', link: '/plugins/examples/docker-deployment' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/wolfware-labs/moonlit' }
    ],

    footer: {
      message: 'Released under the MIT License. | Made with ❤️ (and a lot of ☕) by the <a href="https://wolfware.dev" target="_blank">Wolfware</a> team',
      copyright: 'Copyright © Wolfware LLC'
    }
  }
})
