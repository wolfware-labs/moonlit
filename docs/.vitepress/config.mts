import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Moonlit",
  description: "A powerful build and release pipeline tool built on .NET",
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/' },
      { text: 'Reference', link: '/reference/' },
      { text: 'Plugins', link: '/plugins/' }
    ],

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
            { text: 'Middleware Pipeline', link: '/guide/advanced/middleware' }
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
      { icon: 'github', link: 'https://github.com/wolfware/moonlit' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © Wolfware LLC'
    }
  }
})
