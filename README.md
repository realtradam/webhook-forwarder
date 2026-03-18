# webhook-forwarder

Minimal Rust HTTP server that forwards GitHub webhook requests to a Dokploy instance that is not publicly accessible.

## How It Works

| Incoming Request | Forwarded To |
|---|---|
| `POST /<token>` | `{DOKPLOY_BASE_URL}/api/deploy/<token>` |
| `POST /compose/<token>` | `{DOKPLOY_BASE_URL}/api/deploy/compose/<token>` |

All headers and the request body are forwarded. Dokploy's response is returned to the caller.

## Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `DOKPLOY_BASE_URL` | `http://100.102.55.49:3000` | Internal Dokploy panel URL |
| `PORT` | `8080` | Listen port |

## Deploy on Dokploy

### 1. Create the Application

1. Go to **Dashboard → Projects** → create or select a project
2. Click **"+ Create Service"** → **"Application"**
3. Name it (e.g. `webhook-forwarder`)

### 2. Configure Source

1. Go to the **Provider** tab
2. Select **GitHub** and choose this repository
3. Set branch to `main`
4. Build path: `/` (root)

### 3. Configure Build

1. Go to the **Build** tab
2. Set build type to **Dockerfile**
3. Dockerfile path: `Dockerfile`

### 4. Set Environment Variables

1. Go to the **Environment** tab
2. Add:
   ```
   DOKPLOY_BASE_URL=http://172.17.0.1:3000
   ```
   This uses the Docker bridge gateway IP to reach Dokploy on the host. If this doesn't work, try your Tailscale IP or `host.docker.internal:3000`.

### 5. Add Domain

1. Go to the **Domains** tab
2. Add a domain:
   - **Host**: `webhook.catgirls.rodeo` (or your domain)
   - **Port**: `8080`
   - **HTTPS**: On
   - **Certificate**: `letsencrypt`
3. Ensure your DNS has an A record pointing the domain to your server

### 6. Deploy

Click **Deploy** and monitor the **Deployments** tab for build logs.

## Configure GitHub Webhooks

For each repo you want to auto-deploy via Dokploy:

1. Find the deploy token in Dokploy's application/compose settings
2. Go to your GitHub repo → **Settings → Webhooks → Add webhook**
3. Set the payload URL:
   - For applications: `https://webhook.catgirls.rodeo/<token>`
   - For compose: `https://webhook.catgirls.rodeo/compose/<token>`
4. Content type: `application/json`
5. Events: **Just the push event**

## Local Development

```bash
# Build and run
cargo run

# Test (in another terminal)
chmod +x bin/test.sh
bin/test.sh
```

## Networking Troubleshooting

If the forwarder can't reach Dokploy (`502 Bad Gateway`):

- **Docker bridge**: Try `DOKPLOY_BASE_URL=http://172.17.0.1:3000`
- **Tailscale IP**: Try `DOKPLOY_BASE_URL=http://100.102.55.49:3000`
- **Host network mode**: As a last resort, deploy with `network_mode: host` (requires compose deployment instead of application)
