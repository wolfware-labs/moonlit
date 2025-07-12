FROM node:22-alpine AS build

WORKDIR /app

ARG ALGOLIA_APP_ID=YOUR_APP_ID
ARG ALGOLIA_API_KEY=YOUR_API_KEY
ARG ALGOLIA_INDEX_NAME=moonlit

ENV ALGOLIA_APP_ID=$ALGOLIA_APP_ID \
    ALGOLIA_API_KEY=$ALGOLIA_API_KEY \
    ALGOLIA_INDEX_NAME=$ALGOLIA_INDEX_NAME

COPY package*.json ./

RUN npm ci

COPY . .

RUN npm run docs:build

FROM nginx:alpine

COPY --from=build /app/.vitepress/dist /usr/share/nginx/html

EXPOSE 80

RUN echo '#!/bin/sh\n\
echo "Starting Moonlit documentation server..."\n\
echo "Algolia search is configured with the values provided during build time."\n\
exec nginx -g "daemon off;"\n' > /docker-entrypoint.sh && \
chmod +x /docker-entrypoint.sh

ENTRYPOINT ["/docker-entrypoint.sh"]
