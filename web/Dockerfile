# Static build served by a plain nginx — the gateway (docker/nginx) proxies
# to this container's port 80, same as any other service.
FROM node:22-slim AS builder
WORKDIR /build
COPY package.json ./
RUN npm install
COPY . .
ARG VITE_API_BASE_URL=/api
ENV VITE_API_BASE_URL=${VITE_API_BASE_URL}
RUN npm run build

FROM nginx:1.27-alpine AS runtime
COPY --from=builder /build/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
