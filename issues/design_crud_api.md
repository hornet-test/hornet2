# Design for Local Arazzo File CRUD API

## Overview
This document outlines the design for adding CRUD (Create, Read, Update, Delete) capabilities to the Hornet2 API server to allow modification of the local Arazzo specification file.

## Goals
- Allow the frontend to retrieve the complete Arazzo specification.
- Allow the frontend to modify the Arazzo specification (update, add, delete workflows).
- Persist changes to the local file system.

## Proposed API Endpoints

### 1. Get Full Specification
- **Endpoint**: `GET /api/spec`
- **Description**: Returns the complete Arazzo specification structure.
- **Response**: `200 OK` with JSON body containing the `ArazzoSpec`.

### 2. Update Full Specification
- **Endpoint**: `PUT /api/spec`
- **Description**: Replaces the entire Arazzo specification with the provided JSON.
- **Body**: JSON representation of `ArazzoSpec`.
- **Behavior**:
    1. Validates the received specification.
    2. Serializes it to YAML.
    3. Overwrites the file at `arazzo_path`.
- **Response**: `200 OK` if successful, `400 Bad Request` if validation fails.

### 3. Get Workflow
- **Endpoint**: `GET /api/workflows/:workflow_id`
- **Description**: Returns a single workflow by its ID.
- **Response**: `200 OK` with `Workflow` object, or `404 Not Found`.

### 4. Create/Update Workflow
- **Endpoint**: `PUT /api/workflows/:workflow_id`
- **Description**: Creates or updates a workflow with the given ID.
- **Body**: JSON representation of `Workflow`.
- **Behavior**:
    1. Loads the current Arazzo spec.
    2. Checks if a workflow with `workflow_id` exists.
    3. If it exists, updates it. If not, adds it to the list.
    4. Validates the updated spec.
    5. Saves to file.
- **Response**: `200 OK` with the updated workflow.

### 5. Delete Workflow
- **Endpoint**: `DELETE /api/workflows/:workflow_id`
- **Description**: Deletes a workflow by its ID.
- **Behavior**:
    1. Loads the current Arazzo spec.
    2. Removes the workflow with `workflow_id`.
    3. Saves to file.
- **Response**: `200 OK`, or `404 Not Found` if it didn't exist.

## Implementation Details

### Data Persistence
- The server currently reads the Arazzo file on every request.
- For write operations, we will read, modify, and then write back to the same path using `fs::write`.
- We need to ensure that `serde_yaml` is used for serialization to maintain the file format.

### Validation
- Reuse existing `validate()` methods in `ArazzoSpec`, `Workflow`, etc.
- Ensure that modifying one part doesn't break references (though deep referential integrity checks might be out of scope for MVP, basic ID uniqueness checks are already in place).

### Error Handling
- Return appropriate HTTP status codes (400 for validation errors, 404 for not found, 500 for IO errors).
- Return descriptive error messages.

## Security Considerations
- This API allows modifying files on the server. Since this is a local development tool (implied by "local Arazzo file"), we assume the user has access to the file system anyway.
- Basic path traversal checks are irrelevant here as the path is fixed at startup.

## Plan
1. Implement `save_arazzo` helper function in `loader` module.
2. Implement `GET /api/spec` endpoint.
3. Implement `PUT /api/spec` endpoint.
4. Implement `GET`, `PUT`, `DELETE` for `/api/workflows/:workflow_id`.
5. Register new routes in `server/mod.rs`.
6. Add tests for the new endpoints.
