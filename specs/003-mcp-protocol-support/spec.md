# Feature Specification: MCP Protocol Version Negotiation and Conversion Layer

**Feature Branch**: `003-mcp-protocol-support`
**Created**: 2025-10-12
**Status**: Draft
**Input**: User description: "mcp protocol support generate a conversion layer between our proxy endpoints and the mcp endpoints (whether sse, http, ws, or stdio) that supports the different models. the first initialization call should negotiate the version of the protocol to use with the mcp server. then the notifications/initialized will be sent from there the different protocol interface would have been selected for that mcp server. create a mcp interface for 2025-06-18, 2025-03-26, and 2024-11-05"

## User Scenarios & Testing

### User Story 1 - Connect to MCP Servers with Different Protocol Versions (Priority: P1)

When the proxy connects to backend MCP servers, each server may support a different version of the MCP specification (2024-11-05, 2025-03-26, or 2025-06-18). The proxy automatically detects each server's protocol version during initialization and adapts its communication accordingly, ensuring all servers can participate regardless of their specification version.

**Why this priority**: This is the foundational capability that enables multi-version support. Without this, servers using different protocol versions cannot connect to the proxy, directly addressing the current issue where servers crash or fail to provide tools due to protocol mismatches.

**Independent Test**: Can be fully tested by configuring backend servers with different protocol versions (one using 2024-11-05, one using 2025-03-26, one using 2025-06-18) and verifying that the proxy successfully connects to all of them and aggregates their tools into a single unified list.

**Acceptance Scenarios**:

1. **Given** a backend server supports protocol version 2024-11-05, **When** the proxy sends an initialize request, **Then** the proxy detects the version from the server's response and uses 2024-11-05 communication patterns for all subsequent requests
2. **Given** a backend server supports protocol version 2025-03-26, **When** the proxy sends an initialize request, **Then** the proxy detects the version and uses 2025-03-26 communication patterns
3. **Given** a backend server supports protocol version 2025-06-18, **When** the proxy sends an initialize request, **Then** the proxy detects the version and uses 2025-06-18 communication patterns
4. **Given** multiple backend servers with different protocol versions, **When** the proxy initializes all connections, **Then** each server communicates using its respective protocol version without conflicts

---

### User Story 2 - Reliable Server Initialization Without Crashes (Priority: P1)

Backend MCP servers require a specific initialization handshake sequence. The proxy follows the correct protocol sequence for each server's version: send initialize request, receive response, send initialized notification, then proceed with normal operations. This prevents servers from crashing due to receiving requests before initialization completes.

**Why this priority**: This directly solves the critical production issue where 5 out of 9 servers crash because they receive requests before the initialization handshake completes, resulting in only 42 tools instead of the expected 76-86 tools.

**Independent Test**: Can be tested by connecting to a backend server, monitoring the initialization sequence in logs, and verifying that: (1) initialize request is sent first, (2) initialized notification is sent after receiving the response, (3) no other requests are sent until after the notification, (4) the server successfully provides its tools without crashing.

**Acceptance Scenarios**:

1. **Given** a backend server is starting up, **When** the proxy initiates connection, **Then** the proxy sends initialize request, waits for response, sends initialized notification, and only then sends other requests like tools/list
2. **Given** a slow-initializing backend server (30+ seconds to respond), **When** the proxy connects, **Then** the proxy waits for initialization to complete before sending any other requests
3. **Given** a backend server is initialized, **When** the proxy sends tools/list, **Then** the server responds successfully without crashing or closing the connection
4. **Given** multiple clients connect to the proxy simultaneously, **When** the proxy forwards their requests to backend servers, **Then** only fully-initialized servers receive requests

---

### User Story 3 - Automatic Protocol Translation for Client Requests (Priority: P2)

When a client (like Claude Code) sends a request to the proxy using the current protocol version, the proxy translates that request to match each backend server's protocol version before forwarding it. The proxy then translates responses from backend servers back to the client's expected protocol version.

**Why this priority**: This enables seamless multi-version operation where clients and servers don't need to know about version differences. It's P2 because it builds on P1's version detection and is needed for full protocol compatibility.

**Independent Test**: Can be tested by sending a request formatted for protocol version 2025-03-26 to the proxy, forwarding it to a server using protocol version 2024-11-05, and verifying that: (1) the request is successfully translated, (2) the server understands and responds, (3) the response is translated back to 2025-03-26 format for the client.

**Acceptance Scenarios**:

1. **Given** a client sends a 2025-03-26 format request and the backend server uses 2024-11-05, **When** the proxy forwards the request, **Then** the request is translated to 2024-11-05 format and the response is translated back to 2025-03-26
2. **Given** protocol format differences (e.g., parameter names, structure changes), **When** translating between versions, **Then** all data is preserved and correctly mapped to the target version's format
3. **Given** a feature exists in a newer protocol but not in an older one, **When** a request uses that feature with an older backend, **Then** the proxy handles it gracefully with appropriate fallback behavior

---

### User Story 4 - Version Compatibility Reporting (Priority: P3)

The proxy provides visibility into which protocol version each backend server is using and any compatibility issues. This helps operators understand the protocol landscape of their backend servers and troubleshoot version-related issues.

**Why this priority**: This is operational visibility and troubleshooting support. It's P3 because the core functionality (P1, P2) can work without it, but it significantly improves the operator experience.

**Independent Test**: Can be tested by connecting servers with mixed protocol versions and checking the proxy's status endpoints or logs to see: (1) which version each server reported, (2) whether any version-related warnings or errors occurred, (3) which features are available for each server based on its version.

**Acceptance Scenarios**:

1. **Given** backend servers are connected with different protocol versions, **When** an operator checks server status, **Then** each server's protocol version is clearly displayed
2. **Given** a protocol version mismatch causes a feature to be unavailable, **When** the issue occurs, **Then** the proxy logs a clear warning explaining which server and which feature is affected
3. **Given** servers using deprecated protocol versions, **When** they connect, **Then** operators receive notifications about version support timelines

---

### Edge Cases

- What happens when a backend server reports an unsupported protocol version (e.g., 2026-01-01)?
- How does the system handle a server that changes its reported protocol version after initialization?
- What happens when a backend server's initialize response is malformed or missing the protocol version field?
- How does the proxy handle a backend server that disconnects during the initialization handshake?
- What happens when a client requests a feature that doesn't exist in a particular backend server's protocol version?
- How does the system handle backward-incompatible changes between protocol versions?
- What happens when the initialized notification fails to send or times out?

## Requirements

### Functional Requirements

- **FR-001**: System MUST negotiate protocol version with each backend MCP server during the initialize handshake
- **FR-002**: System MUST support three protocol versions: 2024-11-05, 2025-03-26, and 2025-06-18
- **FR-003**: System MUST extract the protocol version from each server's initialize response
- **FR-004**: System MUST send the initialized notification only after receiving the initialize response from a backend server
- **FR-005**: System MUST prevent any non-initialization requests (tools/list, resources/list, etc.) from being sent to a backend server until after the initialized notification is sent
- **FR-006**: System MUST maintain per-server protocol version state for the lifetime of each connection
- **FR-007**: System MUST translate requests from the proxy's protocol format to each backend server's protocol version format
- **FR-008**: System MUST translate responses from each backend server's protocol version format to the proxy's protocol format
- **FR-009**: System MUST handle protocol differences in request/response structure between versions (parameter names, required fields, data types)
- **FR-010**: System MUST handle protocol differences in notification formats between versions
- **FR-011**: System MUST work with all transport types (stdio, HTTP-SSE, WebSocket) regardless of protocol version
- **FR-012**: System MUST log the detected protocol version for each backend server connection
- **FR-013**: System MUST provide a fallback behavior when a backend server reports an unsupported protocol version
- **FR-014**: System MUST re-negotiate protocol version if a backend server connection is re-established
- **FR-015**: System MUST ensure the initialization sequence (initialize → response → initialized notification) completes before marking a server as "ready" for requests

### Key Entities

- **Protocol Version**: Represents a specific version of the MCP specification (2024-11-05, 2025-03-26, or 2025-06-18) with its associated message formats, required fields, and behavior
- **Protocol Adapter**: A conversion layer that translates messages between different protocol versions, containing version-specific serialization/deserialization logic
- **Server Connection State**: Tracks whether a backend server has completed initialization, which protocol version it uses, and whether it's ready to receive requests
- **Initialization Handshake**: The sequence of messages (initialize request → initialize response → initialized notification) that establishes the protocol version and readiness state

## Success Criteria

### Measurable Outcomes

- **SC-001**: All backend MCP servers (9+ servers) successfully connect to the proxy and provide their tools without crashes or connection closures
- **SC-002**: The proxy exposes the full expected tool count (76-86 tools) from all connected backend servers, up from the current 42 tools
- **SC-003**: Backend servers with different protocol versions (2024-11-05, 2025-03-26, 2025-06-18) operate simultaneously without conflicts or errors
- **SC-004**: Zero instances of "Received request before initialization was complete" errors from backend servers after implementation
- **SC-005**: The initialization handshake for each backend server completes within 60 seconds under normal conditions
- **SC-006**: 100% of tools/list requests to fully-initialized backend servers succeed (up from current 44% success rate: 4 out of 9 servers)
- **SC-007**: Backend servers that previously failed to provide tools (fetch, git, time, serena, playwright) now successfully provide all their tools
- **SC-008**: The proxy handles requests from clients using the current protocol version while backend servers use any of the three supported versions

## Out of Scope

The following are explicitly excluded from this feature:

- Adding support for protocol versions older than 2024-11-05 or newer than 2025-06-18
- Implementing features from newer protocol versions that don't exist in older versions (only translation of existing features)
- Upgrading or modifying backend MCP servers themselves
- Changing the protocol version used by clients connecting to the proxy
- Implementing a protocol version upgrade/migration path for backend servers
- Support for custom or proprietary protocol extensions

## Assumptions

- Backend MCP servers correctly report their supported protocol version in the initialize response
- The three specified protocol versions (2024-11-05, 2025-03-26, 2025-06-18) have publicly documented specifications available
- Protocol differences between versions are limited to message format/structure changes rather than fundamental architectural changes
- Backend servers maintain consistent protocol version for the lifetime of a connection
- The current issues (server crashes, missing tools) are caused by protocol version mismatches and initialization sequence errors
- All three protocol versions use the same initialize → initialized notification pattern, with only format differences

## Dependencies

- Access to official MCP specification documentation for versions 2024-11-05, 2025-03-26, and 2025-06-18
- Ability to test with backend servers using each of the three protocol versions
- Understanding of the specific message format differences between the three protocol versions

## Risks

- **Protocol Complexity**: If protocol versions have significant incompatibilities beyond message format, translation may not be feasible
  - *Mitigation*: Analyze specification differences early and identify any blocking incompatibilities

- **Performance Overhead**: Translation layer may add latency to every request/response
  - *Mitigation*: Design translation logic to be lightweight and cache translation rules where possible

- **Incomplete Specification Documentation**: If specification documents are incomplete or ambiguous, correct translation may be difficult
  - *Mitigation*: Test with real backend servers to verify translation correctness beyond specification

- **Hidden Version-Specific Behaviors**: Backend servers may have undocumented version-specific behaviors beyond the specification
  - *Mitigation*: Comprehensive integration testing with servers using each protocol version
