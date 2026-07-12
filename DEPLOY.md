# Deploy Praxis Live

The repository contains the prebuilt Linux x86_64 backend binary used by the
Docker image (`linux/amd64` in Docker terminology). On the VPS, clone the repository, create `.env` from
`.env.example`, configure it, and start the existing Compose stack:

```bash
cp .env.example .env
docker compose up -d --build
curl --fail http://127.0.0.1:3100/api/health
```

Do not run `docker compose down --volumes`; PostgreSQL, Redis, and uploads use
named volumes.

## Refresh the backend binary

After changing Rust code, rebuild and commit the Linux x86_64 artifact from a
machine with Docker/buildx:

```bash
scripts/build-linux-artifact.sh
```

The VPS Docker build uses this artifact and does not install Rust or compile the
backend. The Vite frontend continues to build in Docker.
