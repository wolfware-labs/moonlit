# Algolia Search Integration

Moonlit documentation uses [Algolia DocSearch](https://docsearch.algolia.com/) to provide powerful search functionality. This guide explains how to set up and maintain the Algolia search index for your documentation.

## How Algolia DocSearch Works

Algolia DocSearch is a free service provided by Algolia that crawls your documentation website and creates a search index. This index is then used to power the search functionality on your site.

## Setting Up Algolia DocSearch

To set up Algolia DocSearch for your Moonlit documentation:

1. Apply for the free DocSearch program at [https://docsearch.algolia.com/apply/](https://docsearch.algolia.com/apply/)
2. Once approved, you'll receive an email with your Algolia credentials (appId, apiKey, and indexName)
3. Update the VitePress configuration in `.vitepress/config.mts` with your credentials:

```js
themeConfig: {
  search: {
    provider: 'algolia',
    options: {
      appId: 'YOUR_APP_ID', // Replace with your Algolia App ID
      apiKey: 'YOUR_API_KEY', // Replace with your Algolia API Key
      indexName: 'moonlit', // Replace with your Algolia Index Name
      placeholder: 'Search documentation',
      translations: {
        button: {
          buttonText: 'Search'
        }
      }
    }
  }
}
```

## Customizing the Search Experience

You can customize the search experience by modifying the options in the configuration:

- **placeholder**: The placeholder text shown in the search box
- **translations**: Customize the text used in the search UI
- **searchParameters**: Additional parameters to pass to Algolia's search API

## Maintaining the Search Index

Algolia DocSearch automatically crawls your documentation website on a regular basis (typically weekly). However, if you need to trigger a re-crawl:

1. Contact the Algolia DocSearch team
2. Provide your website URL and explain why you need a re-crawl

## Troubleshooting

If you encounter issues with the search functionality:

1. Check that your Algolia credentials are correct
2. Ensure your documentation website is publicly accessible
3. Verify that the Algolia crawler can access your site (no robots.txt restrictions)
4. Check the browser console for any errors related to Algolia

## Best Practices

- Keep your documentation well-structured with clear headings
- Use descriptive titles and headings to improve search results
- Avoid using images for important text that should be searchable
- Consider the user's search intent when organizing content