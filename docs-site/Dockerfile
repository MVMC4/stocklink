FROM node:24-alpine AS dependencies

ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH
RUN corepack enable

WORKDIR /workspace/docs-site
COPY docs-site/package.json docs-site/pnpm-lock.yaml docs-site/pnpm-workspace.yaml ./
# The package's postinstall (`fumadocs-mdx`) needs source.config.ts and the
# synced content, which do not exist in this manifest-only layer. It runs in
# the build stage instead, after the sources and Markdown are copied.
RUN pnpm install --frozen-lockfile --ignore-scripts

FROM dependencies AS build
ENV NEXT_STANDALONE=true
WORKDIR /workspace
COPY docs-site ./docs-site
COPY docs ./docs
WORKDIR /workspace/docs-site
# Sync the committed Markdown, generate the Fumadocs index from it, then build.
RUN node scripts/sync-content.mjs \
    && pnpm exec fumadocs-mdx \
    && pnpm exec next build

FROM node:24-alpine AS runtime
ENV NODE_ENV=production
ENV HOSTNAME=0.0.0.0
ENV PORT=3010

WORKDIR /app
RUN addgroup --system --gid 1001 stocklink && \
    adduser --system --uid 1001 --ingroup stocklink stocklink
COPY --from=build --chown=stocklink:stocklink /workspace/docs-site/.next/standalone ./
COPY --from=build --chown=stocklink:stocklink /workspace/docs-site/.next/static ./.next/static
USER stocklink
EXPOSE 3010
CMD ["node", "server.js"]
