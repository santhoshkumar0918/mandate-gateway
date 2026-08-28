FROM oven/bun:1 AS builder
WORKDIR /app
ENV NEXT_PUBLIC_GATEWAY_URL=http://localhost:8000
COPY package.json bun.lock* ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM oven/bun:1-slim
WORKDIR /app
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public
EXPOSE 3000
ENV PORT=3000
ENV NODE_ENV=production
CMD ["bun", "server.js"]
