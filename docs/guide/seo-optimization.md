---
title: SEO Optimization Guide
description: Learn how to optimize your Moonlit documentation pages for search engines
keywords: SEO, search engine optimization, metadata, frontmatter, Moonlit documentation
author: Wolfware Team
---

# SEO Optimization Guide

This guide explains how to optimize your Moonlit documentation pages for search engines.

## Using Frontmatter for SEO

The simplest way to add SEO metadata to your pages is by using frontmatter at the top of your markdown files:

```md
---
title: Your Page Title
description: A detailed description of your page (150-160 characters ideal)
keywords: keyword1, keyword2, keyword3
author: Your Name
image: /path/to/social-share-image.jpg
---
```

These values will be automatically used for:
- Page title
- Meta description
- Meta keywords
- Author information
- Social sharing images (Open Graph and Twitter)

## Using the SEOMetadata Component

For more control, you can use the `SEOMetadata` component directly in your markdown:

```md
<SEOMetadata 
  title="Custom Page Title" 
  description="Custom page description that overrides the frontmatter"
  keywords="custom, keywords, here"
  image="/custom-image.jpg"
  author="Custom Author"
/>

# Your Page Content
```

## SEO Best Practices

1. **Use descriptive titles**: Each page should have a unique, descriptive title (60-70 characters max)
2. **Write compelling descriptions**: Create unique meta descriptions (150-160 characters) that accurately summarize the page content
3. **Use relevant keywords**: Include relevant keywords naturally in your content, headings, and metadata
4. **Structure content with headings**: Use proper heading hierarchy (H1, H2, H3) to structure your content
5. **Add alt text to images**: Always include descriptive alt text for images
6. **Internal linking**: Link to other relevant pages within the documentation
7. **Keep content up-to-date**: Regularly update your content to ensure it remains accurate

## Checking Your SEO

You can use browser developer tools to verify that your SEO metadata is being correctly applied:

1. Right-click on your page and select "View Page Source"
2. Look for the `<meta>` tags in the `<head>` section
3. Verify that your title, description, and other metadata are correctly displayed

For more advanced SEO analysis, consider using tools like Google's Lighthouse, which is built into Chrome DevTools.