### Webhook Forwarder — Implementation Plan

---

### Overview

A minimal Rust HTTP server that accepts GitHub webhook POST requests on a public domain (`webhook.catgirls.rodeo`) and forwards them to Dokploy's internal deploy API. This avoids exposing Dokploy's panel to the public internet.

---

### URL Routing

| Incoming Request | Forwarded To |
|---|---|
| `POST webhook.catgirls.rodeo/<token>` | `POST {DOKPLOY_BASE_URL}/api/deploy/<token>` |
| `POST webhook.catgirls.rodeo/compose/<token>` | `POST {DOKPLOY_BASE_URL}/api/deploy/compose/<token>` |

All other methods/paths return `405 Method Not Allowed` or `404 Not Found`.

---

### Configuration

| Variable | Default | Description |
|---|---|---|
| `DOKPLOY_BASE_URL` | `http://100.102.55.49:3000` | Internal Dokploy panel URL |
| `PORT` | `8080` | Port the forwarder listens on |

No secrets are stored in this app. The deploy tokens live in GitHub's webhook config and Dokploy.

---

### Behavior

1. Accept only `POST` requests. Return `405` for all other methods.
2. Match paths:
   - `/<token>` → forward to `/api/deploy/<token>`
   - `/compose/<token>` → forward to `/api/deploy/compose/<token>`
   - Everything else → `404`
3. Forward the full request body and all headers from the incoming request to Dokploy.
4. Forward Dokploy's response (status code + body) back to the caller.
5. Log each request to stdout: timestamp, method, path, upstream status code.
6. On upstream connection failure, return `502 Bad Gateway`.

---

### Technology Stack

- **Language**: Rust
- **HTTP server**: `hyper` (minimal async HTTP library, no full framework overhead)
- **HTTP client**: `hyper` + `hyper-util` (reuse the same library for outbound requests)
- **Async runtime**: `tokio` (minimal feature set: `rt`, `net`, `macros`)
- **No other dependencies** unless strictly necessary

This yields a single static binary of ~2–4 MB with musl libc.

---

### File Structure

```
webhook-forwarder/
├── .dockerignore
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── src/
│   └── main.rs
└── .rules/
    └── plan/
        └── webhook-forwarder.md   (this file)
```

Total: ~5 meaningful files. Minimal git repo.

---

### Dockerfile Strategy

Multi-stage build for smallest possible image:

```
Stage 1: rust:alpine (build with musl for static binary)
  - cargo build --release --target x86_64-unknown-linux-musl

Stage 2: scratch (empty image — just the binary)
  - COPY binary from stage 1
  - EXPOSE 8080
  - ENTRYPOINT ["./webhook-forwarder"]
```

Final image: ~3–5 MB total (just the static binary, no OS, no shell).

---

### Networking

**Primary approach (Option A):** Use Docker bridge gateway IP to reach Dokploy.

The container accesses Dokploy via the Docker host's bridge IP (typically `172.17.0.1` on default bridge, or the gateway of whatever network Dokploy uses). Set `DOKPLOY_BASE_URL=http://172.17.0.1:3000` in Dokploy's environment config for this app.

However, the default value is the Tailscale IP `http://100.102.55.49:3000` for direct use if networking allows.

**Fallback (Option B):** If the Docker bridge approach doesn't work (e.g., Dokploy's firewall blocks it), options include:
- `network_mode: host` in compose to give the container access to the host's Tailscale interface
- Running a Tailscale sidecar container
- Adding the container to Dokploy's own Docker network

We'll try Option A first since it's simplest.

---

### Dokploy Deployment

1. Push this repo to GitHub
2. In Dokploy, create a new **Application** (not Compose — single container, no need for compose)
3. Set source to the GitHub repo, branch `main`
4. Build type: **Dockerfile**
5. Environment variable: `DOKPLOY_BASE_URL=http://172.17.0.1:3000` (or whatever internal IP works)
6. Domain: `webhook.catgirls.rodeo`, port `8080`, HTTPS on, letsencrypt
7. Deploy

Then update GitHub webhook URLs for your other repos:
- Application deploys: `https://webhook.catgirls.rodeo/<token>`
- Compose deploys: `https://webhook.catgirls.rodeo/compose/<token>`

---

### Implementation Steps

1. **Initialize Cargo project**: `Cargo.toml` with minimal deps (`hyper`, `tokio`, `http-body-util`, `hyper-util`)
2. **Write `src/main.rs`**:
   - Read `DOKPLOY_BASE_URL` and `PORT` from env (with defaults)
   - Start hyper HTTP server on `0.0.0.0:{PORT}`
   - Route handler:
     - Reject non-POST → 405
     - Parse path: extract `/<token>` or `/compose/<token>`
     - Build upstream URL: `{base}/api/deploy/{path}`
     - Forward headers + body via hyper client
     - Return upstream response to caller
     - Log to stdout
3. **Write `Dockerfile`**: multi-stage, musl static build, `FROM scratch`
4. **Write `.dockerignore`**: exclude `target/`, `.git/`, `.rules/`, `*.md`
5. **Write `.gitignore`**: standard Rust ignores (`/target`)
6. **Test locally**: `cargo run`, then `curl -X POST localhost:8080/test-token`
7. **Push to GitHub, deploy via Dokploy**
8. **Test end-to-end**: update one GitHub repo's webhook URL, push, verify deploy triggers

---

### Resource Footprint Estimate

| Metric | Estimate |
|---|---|
| Docker image size | ~3–5 MB |
| RAM (idle) | ~1–2 MB |
| RAM (under load) | ~3–5 MB |
| CPU (idle) | ~0% |
| Git repo size | < 50 KB (excluding target/) |
