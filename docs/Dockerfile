# Use Node.js LTS as the base image
FROM node:20-alpine as build

# Set working directory
WORKDIR /app

# Define build arguments for Algolia configuration
ARG ALGOLIA_APP_ID=YOUR_APP_ID
ARG ALGOLIA_API_KEY=YOUR_API_KEY
ARG ALGOLIA_INDEX_NAME=moonlit

# Set environment variables from build arguments
ENV ALGOLIA_APP_ID=$ALGOLIA_APP_ID \
    ALGOLIA_API_KEY=$ALGOLIA_API_KEY \
    ALGOLIA_INDEX_NAME=$ALGOLIA_INDEX_NAME

# Copy package.json and package-lock.json
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy the rest of the application
COPY . .

# Build the VitePress site with environment variables available
RUN npm run docs:build

# Use Nginx to serve the static files
FROM nginx:alpine

# Copy the built files from the build stage to Nginx's serve directory
COPY --from=build /app/.vitepress/dist /usr/share/nginx/html

# Copy a custom nginx configuration if needed
# COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port 80
EXPOSE 80

# Create a startup script
RUN echo '#!/bin/sh\n\
echo "Starting Moonlit documentation server..."\n\
echo "Algolia search is configured with the values provided during build time."\n\
exec nginx -g "daemon off;"\n' > /docker-entrypoint.sh && \
chmod +x /docker-entrypoint.sh

# Set the entrypoint
ENTRYPOINT ["/docker-entrypoint.sh"]
