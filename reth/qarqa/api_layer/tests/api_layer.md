# API Layer Testing Documentation

## Overview

The `api_layer` module provides REST API endpoints, CLI tools, and WebSocket interfaces for the QARQA system. This document specifies comprehensive testing for endpoints, authentication, rate limiting, and real-time features.

## What We Are Testing

### 1. REST API Endpoints

#### Fund Flow Endpoints
- **GET /api/v1/fund-flow/{address}**: Single address analysis
- **POST /api/v1/fund-flow/batch**: Multiple address analysis
- **GET /api/v1/fund-flow/{address}/history**: Historical flows
- **GET /api/v1/fund-flow/{address}/summary**: Aggregated stats
- **WebSocket /api/v1/fund-flow/stream**: Real-time updates

#### Transaction Analysis
- **GET /api/v1/transaction/{hash}**: Single transaction
- **POST /api/v1/transaction/simulate**: Simulate transaction
- **GET /api/v1/transaction/{hash}/trace**: Internal transfers
- **POST /api/v1/transaction/batch**: Batch analysis
- **GET /api/v1/transaction/recent**: Latest transactions

#### Network Visualization
- **GET /api/v1/network/{address}**: Build network graph
- **POST /api/v1/network/custom**: Custom network params
- **GET /api/v1/network/{id}/export**: Export formats
- **WebSocket /api/v1/network/live**: Live network updates
- **GET /api/v1/network/patterns**: Pattern detection

### 2. Authentication and Authorization

#### JWT Authentication
- **Token generation**: Valid JWT creation
- **Token validation**: Signature verification
- **Token expiration**: Proper timeout handling
- **Token refresh**: Refresh token flow
- **Token revocation**: Blacklist support

#### Permission System
- **Role-based access**: Admin, user, viewer roles
- **Resource permissions**: Per-endpoint access
- **Rate limit tiers**: Different limits per role
- **API key support**: Alternative auth method
- **OAuth integration**: Third-party auth

### 3. Rate Limiting and Quotas

#### Request Rate Limiting
- **Per-IP limiting**: Default rate limits
- **Per-user limiting**: Authenticated user limits
- **Endpoint-specific**: Different limits per endpoint
- **Burst allowance**: Short-term burst handling
- **Quota tracking**: Daily/monthly quotas

#### Resource Protection
- **Query complexity**: Limit expensive operations
- **Result size limits**: Maximum response size
- **Timeout enforcement**: Request timeout limits
- **Concurrent requests**: Parallel request limits
- **Memory limits**: Per-request memory bounds

### 4. Input Validation and Security

#### Request Validation
- **Parameter types**: Type checking and coercion
- **Required fields**: Missing parameter handling
- **Value ranges**: Min/max enforcement
- **Format validation**: Address, hash formats
- **Injection prevention**: SQL, NoSQL, command injection

#### Security Headers
- **CORS policy**: Proper origin handling
- **CSP headers**: Content security policy
- **Rate limit headers**: X-RateLimit-* headers
- **Security headers**: HSTS, X-Frame-Options
- **API versioning**: Version in headers/path

### 5. Error Handling and Responses

#### Error Responses
- **HTTP status codes**: Correct status usage
- **Error format**: Consistent error structure
- **Error details**: Helpful error messages
- **Validation errors**: Field-level errors
- **Rate limit errors**: Retry-After header

#### Response Formats
- **JSON responses**: Proper serialization
- **Pagination**: Cursor/offset pagination
- **Filtering**: Query parameter filtering
- **Sorting**: Multi-field sorting
- **Field selection**: Sparse fieldsets

### 6. WebSocket and Real-time

#### Connection Management
- **Handshake**: Proper upgrade negotiation
- **Authentication**: Token in connection
- **Heartbeat**: Keep-alive mechanism
- **Reconnection**: Auto-reconnect logic
- **Connection limits**: Max connections per user

#### Message Handling
- **Subscribe/unsubscribe**: Topic management
- **Message routing**: Correct delivery
- **Backpressure**: Handle slow clients
- **Message ordering**: Sequence guarantees
- **Error propagation**: WebSocket errors

## How We Test It

### Unit Tests for Handlers

```rust
#[tokio::test]
async fn test_fund_flow_endpoint_validation() {
    // Arrange
    let app = create_test_app();
    
    // Act - Invalid address
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fund-flow/invalid-address")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    // Assert
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.error.code, "INVALID_ADDRESS");
}
```

### Integration Tests with Full Stack

```rust
#[tokio::test]
async fn test_complete_fund_flow_analysis() {
    // Arrange
    let server = TestServer::new().await;
    let client = TestClient::new(&server);
    let token = client.authenticate("test_user", "test_pass").await;
    
    // Act
    let response = client
        .get("/api/v1/fund-flow/0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    
    // Assert
    assert_eq!(response.status(), 200);
    
    let fund_flows: FundFlowResponse = response.json().await.unwrap();
    assert!(!fund_flows.flows.is_empty());
    assert!(fund_flows.metadata.total_value > 0.0);
}
```

### WebSocket Tests

```rust
#[tokio::test]
async fn test_websocket_real_time_updates() {
    // Arrange
    let server = TestServer::new().await;
    let ws_url = format!("ws://{}/api/v1/fund-flow/stream", server.addr());
    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();
    
    // Act - Subscribe to address
    let subscribe_msg = json!({
        "action": "subscribe",
        "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    });
    write.send(Message::Text(subscribe_msg.to_string())).await.unwrap();
    
    // Simulate new transaction
    server.simulate_transaction().await;
    
    // Assert - Receive update
    let msg = read.next().await.unwrap().unwrap();
    let update: FundFlowUpdate = serde_json::from_str(&msg.to_text().unwrap()).unwrap();
    assert_eq!(update.event_type, "new_flow");
}
```

### Load and Performance Tests

```rust
#[tokio::test]
async fn test_rate_limiting_enforcement() {
    // Arrange
    let server = TestServer::new().await;
    let client = TestClient::new(&server);
    
    // Act - Send requests up to limit
    for i in 0..100 {
        let response = client
            .get(&format!("/api/v1/transaction/{}", test_tx_hash(i)))
            .send()
            .await
            .unwrap();
        
        if i < 99 {
            assert_eq!(response.status(), 200);
        } else {
            // Assert - 100th request rate limited
            assert_eq!(response.status(), 429);
            assert!(response.headers().contains_key("Retry-After"));
        }
    }
}
```

## Test Cases

### Endpoint Testing Matrix

| Endpoint | Valid Input | Invalid Input | Auth Required | Rate Limit | Cache |
|----------|------------|---------------|---------------|------------|-------|
| GET /fund-flow/{address} | ✓ | ✓ | ✓ | 100/min | 5 min |
| POST /fund-flow/batch | ✓ | ✓ | ✓ | 10/min | No |
| GET /transaction/{hash} | ✓ | ✓ | ✗ | 1000/min | 1 hour |
| GET /network/{address} | ✓ | ✓ | ✓ | 50/min | 10 min |
| WS /network/live | ✓ | ✓ | ✓ | 10 conn | No |

### Authentication Test Cases

1. **Valid Authentication**
   - Correct credentials
   - Valid JWT token
   - Valid API key
   - OAuth token
   - Session cookie

2. **Invalid Authentication**
   - Wrong password
   - Expired token
   - Malformed JWT
   - Revoked API key
   - Invalid OAuth token

3. **Permission Tests**
   - Admin endpoints
   - User restrictions
   - Read-only access
   - Resource ownership
   - Cross-tenant access

### Input Validation Tests

1. **Address Parameters**
   - Valid Ethereum address
   - Invalid format
   - SQL injection attempt
   - XSS in address
   - Unicode characters

2. **Numeric Parameters**
   - Valid ranges
   - Negative values
   - Overflow values
   - Non-numeric input
   - Scientific notation

3. **Complex Queries**
   - Filter combinations
   - Sort parameters
   - Pagination limits
   - Field selection
   - Date ranges

### Error Response Tests

1. **Client Errors (4xx)**
   - 400 Bad Request
   - 401 Unauthorized
   - 403 Forbidden
   - 404 Not Found
   - 429 Too Many Requests

2. **Server Errors (5xx)**
   - 500 Internal Error
   - 502 Bad Gateway
   - 503 Service Unavailable
   - 504 Gateway Timeout
   - Database errors

## Test Data

### Test Users and Tokens
```rust
pub const TEST_USERS: &[(&str, &str, &str)] = &[
    ("admin", "admin_pass", "admin"),
    ("user", "user_pass", "user"),
    ("viewer", "viewer_pass", "viewer"),
];

pub fn generate_test_jwt(user: &str, role: &str) -> String {
    // Generate valid JWT for testing
}
```

### Test Requests
```rust
pub fn test_requests() -> Vec<TestRequest> {
    vec![
        TestRequest {
            method: "GET",
            path: "/api/v1/fund-flow/0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            body: None,
            expected_status: 200,
        },
        TestRequest {
            method: "POST",
            path: "/api/v1/transaction/simulate",
            body: Some(json!({
                "from": "0xSender",
                "to": "0xReceiver",
                "value": "1000000000000000000",
                "data": "0x"
            })),
            expected_status: 200,
        },
    ]
}
```

## Expected Test Outcomes

### Response Time SLAs
- Simple queries: < 100ms (p95)
- Complex analysis: < 1s (p95)
- Network building: < 2s (p95)
- Batch operations: < 5s (p95)
- WebSocket latency: < 50ms

### Reliability Targets
- API availability: 99.9% uptime
- Error rate: < 0.1% 5xx errors
- Success rate: > 99% for valid requests
- Rate limit accuracy: 100% enforcement
- Auth validation: Zero false positives

### Security Requirements
- No SQL injection vulnerabilities
- No XSS vulnerabilities
- Proper CORS enforcement
- Rate limit bypass: Not possible
- Token security: No leaks

## Running the Tests

```bash
# Run all API tests
cargo test -p qarqa-api-layer

# Run integration tests with server
cargo test -p qarqa-api-layer --features integration -- --test-threads=1

# Run load tests
cargo test -p qarqa-api-layer load_test_ -- --ignored

# Run with test server
TEST_SERVER=1 cargo test -p qarqa-api-layer

# Run security tests
cargo test -p qarqa-api-layer security_ -- --nocapture
```

## Test Environment

### Test Server Configuration
```rust
pub fn test_server_config() -> ServerConfig {
    ServerConfig {
        port: 0, // Random port
        workers: 2,
        max_connections: 100,
        rate_limit: RateLimitConfig {
            requests_per_minute: 100,
            burst_size: 10,
        },
        auth: AuthConfig {
            jwt_secret: "test_secret",
            token_expiry: Duration::from_secs(3600),
        },
    }
}
```

### Mock Services
```rust
impl MockDatabase {
    pub async fn new() -> Self {
        // In-memory database for testing
    }
}

impl MockCache {
    pub fn new() -> Self {
        // In-memory cache for testing
    }
}
```

## Test Maintenance

- Update API documentation with tests
- Add tests for new endpoints
- Monitor test coverage (> 90%)
- Review security tests quarterly
- Performance baseline updates