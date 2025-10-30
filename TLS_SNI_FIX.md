# TLS SNI Extraction Fix for Claude API Proxy

## Problem

The proxy was experiencing TLS handshake failures with the error:
```
TLS connect error: error:0A000126:SSL routines::unexpected eof while reading
Send failure: Broken pipe
```

The root cause was that the SNI (Server Name Indication) was **hardcoded** to `"api.anthropic.com"` instead of being extracted from the TLS ClientHello message. This meant:
1. The proxy couldn't serve the correct certificate for the requested domain
2. Clients would reject the certificate as it didn't match the requested hostname
3. The TLS handshake would fail before any HTTP traffic could be intercepted

## Solution

Implemented proper SNI extraction using **`tokio_rustls::LazyConfigAcceptor`**, which is the idiomatic way to:
1. Read the TLS ClientHello message asynchronously
2. Extract the SNI field from the ClientHello
3. Dynamically select the appropriate ServerConfig with the correct certificate
4. Complete the TLS handshake with the domain-specific certificate

## Implementation Details

### Key Changes in `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/src/claude_proxy/proxy_server.rs`:

#### 1. Replaced Manual Acceptor with LazyConfigAcceptor

**Before:**
```rust
// Hardcoded domain - WRONG!
let domain = "api.anthropic.com";

// Get config before knowing the actual domain
let server_config = self.tls_handler.get_server_config(domain)?;

// Accept with wrong certificate
let acceptor = TlsAcceptor::from(server_config);
let tls_stream = acceptor.accept(stream).await?;
```

**After:**
```rust
// Use LazyConfigAcceptor to read ClientHello
let acceptor = LazyConfigAcceptor::new(Acceptor::default(), stream);
let start = acceptor.await?;

// Extract actual SNI from ClientHello
let domain = {
    let client_hello = start.client_hello();
    client_hello
        .server_name()
        .ok_or_else(|| ProxyError::Tls("No SNI in ClientHello"))?
        .to_string()
};

// Get config for the ACTUAL requested domain
let server_config = self.tls_handler.get_server_config(&domain)?;

// Complete handshake with correct certificate
let tls_stream = start.into_stream(server_config).await?;
```

#### 2. Updated Imports

Added:
```rust
use rustls::server::Acceptor;
use tokio_rustls::LazyConfigAcceptor;
```

Removed:
```rust
use tokio_rustls::TlsAcceptor;  // No longer needed
```

## How LazyConfigAcceptor Works

1. **Lazy Configuration**: It delays providing the ServerConfig until after the ClientHello is read
2. **Async SNI Extraction**: Reads the ClientHello asynchronously without blocking
3. **Dynamic Certificate Selection**: Allows selecting the appropriate certificate based on the actual SNI value
4. **Zero-Copy Efficiency**: Avoids buffering by working directly with the TCP stream

## Benefits

✅ **Proper SNI Handling**: Extracts the actual hostname from the TLS handshake
✅ **Dynamic Certificate Selection**: Serves the correct certificate for each domain
✅ **MITM Proxy Capability**: Can intercept HTTPS traffic for multiple domains
✅ **Idiomatic Rust**: Uses the recommended tokio-rustls API pattern
✅ **Error Handling**: Properly handles cases where SNI is missing or invalid

## Testing

The fix can be tested by:

1. **Starting the proxy:**
   ```bash
   cargo run -- --config your-config.yaml
   ```

2. **Configuring a client to use the proxy** (e.g., via HTTPS_PROXY environment variable)

3. **Making a request to `api.anthropic.com`:**
   ```bash
   curl --proxy https://localhost:8443 https://api.anthropic.com/v1/...
   ```

4. **Verifying in logs:**
   ```
   DEBUG Extracted SNI from ClientHello domain="api.anthropic.com"
   DEBUG TLS handshake completed successfully domain="api.anthropic.com"
   ```

## Related Files

- `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/src/claude_proxy/proxy_server.rs` - Main fix
- `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/src/claude_proxy/tls_handler.rs` - Certificate generation (unchanged)

## References

- [rustls Acceptor API](https://docs.rs/rustls/latest/rustls/server/struct.Acceptor.html)
- [tokio-rustls LazyConfigAcceptor](https://docs.rs/tokio-rustls/latest/tokio_rustls/struct.LazyConfigAcceptor.html)
- [RFC 6066 - Server Name Indication](https://datatracker.ietf.org/doc/html/rfc6066#section-3)
