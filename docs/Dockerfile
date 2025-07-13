FROM node:22-alpine AS build

WORKDIR /app

# Install git for VitePress
RUN apk add --no-cache git

COPY package*.json ./

RUN npm ci

COPY . .

RUN npm run docs:build

FROM nginx:alpine

COPY --from=build /app/.vitepress/dist /app
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

RUN printf '#!/bin/sh\necho "Starting Moonlit documentation server..."\necho "Algolia search is configured with the values provided during build time."\nexec nginx -g "daemon off;"\n' > /docker-entrypoint.sh && \
chmod +x /docker-entrypoint.sh

ENTRYPOINT ["/docker-entrypoint.sh"]
