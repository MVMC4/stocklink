# StockLink gateway image — a pinned Nginx build with the routing config
# baked in (never bind-mounted), so it can be used as an immutable image in
# staging/prod exactly like the api/web images.
#
# GATEWAY_CONF selects the routing file:
#   docker/nginx/gateway.conf      demo, staging, production (no documentation)
#   docker/nginx/gateway.dev.conf  dev only (adds /docs, /_next/, /api/search)
#
# Build (context must be the repo root):
#   docker build -f docker/nginx/Dockerfile -t stocklink/gateway:staging .
#   docker build -f docker/nginx/Dockerfile \
#     --build-arg GATEWAY_CONF=docker/nginx/gateway.dev.conf -t stocklink/gateway:dev .
FROM nginx:1.27-alpine

ARG GATEWAY_CONF=docker/nginx/gateway.conf
COPY ${GATEWAY_CONF} /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=10s --timeout=3s --retries=5 \
    CMD wget -qO- http://127.0.0.1/gw-health || exit 1
