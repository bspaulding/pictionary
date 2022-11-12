FROM node:lts as frontend
WORKDIR /frontend
COPY frontend/package.json package.json
COPY frontend/package-lock.json package-lock.json
RUN npm ci
COPY frontend/index.html /frontend/index.html
COPY frontend/src src
COPY frontend/public public
COPY frontend/vite.config.js vite.config.js
COPY frontend/jsconfig.json jsconfig.json
RUN npm run build

FROM rust:1.64 as backend
WORKDIR /backend
COPY backend /backend
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev
COPY --from=backend /usr/local/cargo/bin/pictionary /usr/local/bin/pictionary
COPY --from=frontend /frontend/dist /app/frontend/dist
WORKDIR /app/backend
EXPOSE 80
ENV PORT=80
CMD ["pictionary"]
