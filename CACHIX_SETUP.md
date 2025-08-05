# Cachix Setup for MCP Rust Proxy

This document explains how to set up Cachix for the MCP Rust Proxy project to enable fast, cached builds in GitHub Actions.

## Prerequisites

1. Create a Cachix account at https://app.cachix.org
2. Create a new binary cache named `mcp-rust-proxy` (or choose your own name)

## Setup Steps

### 1. Generate Cachix Auth Token

1. Go to https://app.cachix.org/personal-auth-tokens
2. Create a new token with "Write" permissions for your cache
3. Copy the token

### 2. Add GitHub Secret

1. Go to your GitHub repository settings
2. Navigate to Settings → Secrets and variables → Actions
3. Add a new repository secret:
   - Name: `CACHIX_AUTH_TOKEN`
   - Value: The token you generated

### 3. Update Workflow

If you're using a different cache name than `mcp-rust-proxy`, update the `CACHIX_CACHE` environment variable in `.github/workflows/release-nix.yml`:

```yaml
env:
  CACHIX_CACHE: your-cache-name
```

### 4. Test the Setup

1. Push a tag to trigger the release workflow:
   ```bash
   git tag v0.0.2-test
   git push origin v0.0.2-test
   ```

2. Monitor the GitHub Actions run
3. Check your Cachix cache at `https://app.cachix.org/cache/your-cache-name`

## Benefits

With Cachix properly configured, you'll see:

- **First build**: ~20-30 minutes (builds everything from scratch)
- **Subsequent builds**: ~5-8 minutes (uses cached dependencies)
- **Rebuild after Cargo.toml change**: ~10-15 minutes (only rebuilds changed dependencies)

## Local Development

Developers can also use the cache locally:

```bash
# Install cachix
nix-env -iA cachix -f https://cachix.org/api/v1/install

# Configure to use the cache
cachix use mcp-rust-proxy

# Now builds will pull from cache when available
nix build
```

## Troubleshooting

### Authentication Errors

If you see authentication errors in GitHub Actions:
1. Verify the `CACHIX_AUTH_TOKEN` secret is set correctly
2. Check that the token hasn't expired
3. Ensure the token has write permissions

### Cache Misses

If builds aren't using the cache:
1. Check that the cache name matches in all configurations
2. Verify that previous builds successfully pushed to the cache
3. Look for any Nix evaluation differences between builds

### Storage Limits

Free Cachix accounts have a 5GB storage limit. To manage storage:
1. Set up garbage collection rules in Cachix settings
2. Consider upgrading to a paid plan for larger projects
3. Periodically clean old/unused artifacts