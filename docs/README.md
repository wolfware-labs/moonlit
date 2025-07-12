# Moonlit Documentation Docker Setup

This README explains how to build and run the Moonlit documentation site using Docker.

## Prerequisites

- Docker installed on your system
- Algolia account and API credentials (if you want to enable search functionality)

## Building the Docker Image

You can build the Docker image with default Algolia configuration:

```bash
docker build -t moonlit-docs .
```

Or you can provide your own Algolia credentials during build time:

```bash
docker build \
  --build-arg ALGOLIA_APP_ID=YOUR_ACTUAL_APP_ID \
  --build-arg ALGOLIA_API_KEY=YOUR_ACTUAL_API_KEY \
  --build-arg ALGOLIA_INDEX_NAME=YOUR_INDEX_NAME \
  -t moonlit-docs .
```

## Running the Docker Container

Once the image is built, you can run it with:

```bash
docker run -p 8080:80 moonlit-docs
```

This will make the documentation site available at `http://localhost:8080`.

## Algolia Search Configuration

The Algolia search configuration is set during the build process using build arguments. These values are embedded in the static files during the build and cannot be changed at runtime without rebuilding the image.

If you need to update the Algolia configuration, you'll need to rebuild the Docker image with the new values.

## Development

For local development without Docker, you can use:

```bash
npm install
npm run docs:dev
```

This will start a development server with hot reloading.

## Building for Production Without Docker

If you want to build the site without Docker:

```bash
npm install
npm run docs:build
```

The built files will be in the `.vitepress/dist` directory.
