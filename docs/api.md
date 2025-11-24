# CDDE API Documentation

The CDDE Configuration Management Service (CMS) provides a RESTful API for managing the Diameter routing system.

## Interactive API Documentation

When the CMS is running, you can access the interactive Swagger UI documentation at:

```
http://localhost:3000/swagger-ui
```

This provides a complete, interactive API reference with the ability to test endpoints directly.

## Base URL

```
http://localhost:3000/api/v1
```

## Authentication

Currently, the API does not require authentication. This will be added in a future release.

## Endpoints

### Virtual Routers

Virtual Routers (VRs) represent isolated routing domains within CDDE.

#### List Virtual Routers
```http
GET /api/v1/vrs
```

#### Get Virtual Router
```http
GET /api/v1/vrs/{vr_id}
```

#### Create Virtual Router
```http
POST /api/v1/vrs
Content-Type: application/json

{
  "name": "vr-example",
  "realm": "example.com",
  "host": "dra.example.com"
}
```

#### Update Virtual Router
```http
PUT /api/v1/vrs/{vr_id}
Content-Type: application/json

{
  "name": "vr-example",
  "realm": "example.com",
  "host": "dra.example.com"
}
```

#### Delete Virtual Router
```http
DELETE /api/v1/vrs/{vr_id}
```

### Peers

Peers represent Diameter peer connections.

#### List Peers
```http
GET /api/v1/peers
```

#### Get Peer
```http
GET /api/v1/peers/{peer_id}
```

#### Create Peer
```http
POST /api/v1/peers
Content-Type: application/json

{
  "vr_id": "uuid",
  "host": "peer.example.com",
  "realm": "example.com",
  "ip_address": "192.168.1.100",
  "port": 3868,
  "transport": "TCP",
  "is_active": true
}
```

#### Update Peer
```http
PUT /api/v1/peers/{peer_id}
Content-Type: application/json

{
  "vr_id": "uuid",
  "host": "peer.example.com",
  "realm": "example.com",
  "ip_address": "192.168.1.100",
  "port": 3868,
  "transport": "TCP",
  "is_active": true
}
```

#### Delete Peer
```http
DELETE /api/v1/peers/{peer_id}
```

### Dictionaries

Dictionaries define Diameter AVP (Attribute-Value Pair) structures.

#### List Dictionaries
```http
GET /api/v1/dictionaries
```

#### Get Dictionary
```http
GET /api/v1/dictionaries/{dict_id}
```

#### Create Dictionary
```http
POST /api/v1/dictionaries
Content-Type: application/json

{
  "name": "3GPP-S6a",
  "vendor_id": 10415,
  "application_id": 16777251
}
```

#### Update Dictionary
```http
PUT /api/v1/dictionaries/{dict_id}
Content-Type: application/json

{
  "name": "3GPP-S6a",
  "vendor_id": 10415,
  "application_id": 16777251
}
```

#### Delete Dictionary
```http
DELETE /api/v1/dictionaries/{dict_id}
```

#### List Dictionary AVPs
```http
GET /api/v1/dictionaries/{dict_id}/avps
```

#### Create Dictionary AVP
```http
POST /api/v1/dictionaries/{dict_id}/avps
Content-Type: application/json

{
  "code": 1,
  "name": "User-Name",
  "data_type": "UTF8String",
  "flags": "M"
}
```

### Routing Rules

Routing rules determine how Diameter messages are routed to peers.

#### List Routing Rules
```http
GET /api/v1/vrs/{vr_id}/routing-rules
```

#### Get Routing Rule
```http
GET /api/v1/vrs/{vr_id}/routing-rules/{rule_id}
```

#### Create Routing Rule
```http
POST /api/v1/vrs/{vr_id}/routing-rules
Content-Type: application/json

{
  "priority": 100,
  "destination_realm": "example.com",
  "destination_host": "peer.example.com",
  "application_id": 16777251,
  "peer_id": "uuid"
}
```

#### Update Routing Rule
```http
PUT /api/v1/vrs/{vr_id}/routing-rules/{rule_id}
Content-Type: application/json

{
  "priority": 100,
  "destination_realm": "example.com",
  "destination_host": "peer.example.com",
  "application_id": 16777251,
  "peer_id": "uuid"
}
```

#### Delete Routing Rule
```http
DELETE /api/v1/vrs/{vr_id}/routing-rules/{rule_id}
```

### Manipulation Rules

Manipulation rules define AVP modifications using the CDDE DSL.

#### List Manipulation Rules
```http
GET /api/v1/vrs/{vr_id}/manipulation-rules
```

#### Get Manipulation Rule
```http
GET /api/v1/vrs/{vr_id}/manipulation-rules/{rule_id}
```

#### Create Manipulation Rule
```http
POST /api/v1/vrs/{vr_id}/manipulation-rules
Content-Type: application/json

{
  "priority": 100,
  "condition": "avp.Origin-Realm == 'old.example.com'",
  "action": "set avp.Origin-Realm = 'new.example.com'"
}
```

#### Update Manipulation Rule
```http
PUT /api/v1/vrs/{vr_id}/manipulation-rules/{rule_id}
Content-Type: application/json

{
  "priority": 100,
  "condition": "avp.Origin-Realm == 'old.example.com'",
  "action": "set avp.Origin-Realm = 'new.example.com'"
}
```

#### Delete Manipulation Rule
```http
DELETE /api/v1/vrs/{vr_id}/manipulation-rules/{rule_id}
```

## Error Responses

All endpoints return standard HTTP status codes:

- `200 OK`: Successful request
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request data
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Server error

Error responses include a JSON body with details:

```json
{
  "error": "Error message description"
}
```

## Examples

### Creating a Complete Configuration

1. Create a Virtual Router:
```bash
curl -X POST http://localhost:3000/api/v1/vrs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "vr-production",
    "realm": "prod.example.com",
    "host": "dra.prod.example.com"
  }'
```

2. Create a Peer:
```bash
curl -X POST http://localhost:3000/api/v1/peers \
  -H "Content-Type: application/json" \
  -d '{
    "vr_id": "vr-uuid-from-step-1",
    "host": "hss.example.com",
    "realm": "example.com",
    "ip_address": "192.168.1.100",
    "port": 3868,
    "transport": "TCP",
    "is_active": true
  }'
```

3. Create a Routing Rule:
```bash
curl -X POST http://localhost:3000/api/v1/vrs/vr-uuid/routing-rules \
  -H "Content-Type: application/json" \
  -d '{
    "priority": 100,
    "destination_realm": "example.com",
    "application_id": 16777251,
    "peer_id": "peer-uuid-from-step-2"
  }'
```

## Further Information

For the most up-to-date and detailed API documentation, please refer to the Swagger UI at `http://localhost:3000/swagger-ui` when the CMS is running.
