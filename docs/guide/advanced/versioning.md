# Documentation Versioning

Moonlit documentation supports versioning, allowing users to access documentation for different versions of the software.

## How Versioning Works

The versioning system in Moonlit documentation works by:

1. Maintaining a `versions.json` file at the root of the documentation
2. Displaying a version selector in the navigation bar
3. Allowing users to switch between different documentation versions

## Current Structure

The `versions.json` file has the following structure:

```json
{
  "current": "v1.0",
  "versions": [
    {
      "text": "v1.0",
      "link": "/"
    }
  ]
}
```

- `current`: Indicates the current version being displayed
- `versions`: An array of available versions with their display text and links

## Adding a New Version

To add a new version of the documentation:

1. Create a new directory for the version (e.g., `/v2.0/`)
2. Copy the current documentation to the new directory
3. Update the content in the new directory to reflect the new version
4. Update the `versions.json` file to include the new version:

```json
{
  "current": "v2.0",
  "versions": [
    {
      "text": "v2.0",
      "link": "/"
    },
    {
      "text": "v1.0",
      "link": "/v1.0/"
    }
  ]
}
```

5. Move the current documentation to its version-specific directory (e.g., `/v1.0/`)

## Best Practices

- Keep the most recent version at the root level for better SEO
- Use semantic versioning (e.g., v1.0, v1.1, v2.0) for version names
- Update all internal links when creating a new version
- Consider using redirects to handle version changes smoothly