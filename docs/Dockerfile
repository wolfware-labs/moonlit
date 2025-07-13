FROM node:22-alpine AS build

WORKDIR /app

ARG ALGOLIA_APP_ID=YOUR_APP_ID
ARG ALGOLIA_API_KEY=YOUR_API_KEY
ARG ALGOLIA_INDEX_NAME=moonlit

ENV ALGOLIA_APP_ID=$ALGOLIA_APP_ID \
    ALGOLIA_API_KEY=$ALGOLIA_API_KEY \
    ALGOLIA_INDEX_NAME=$ALGOLIA_INDEX_NAME

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
