FROM node:erbium as frontend
WORKDIR /frontend
COPY frontend /frontend
RUN npm install
RUN npm run build

FROM rust:1.42 as backend
WORKDIR /backend
COPY backend /backend
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies
COPY --from=backend /usr/local/cargo/bin/pictionary /usr/local/bin/pictionary
COPY --from=frontend /frontend/public /app/frontend/public
WORKDIR /app/backend
EXPOSE 3000
CMD ["pictionary"]
