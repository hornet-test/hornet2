# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**hornet2** is a document-driven API testing tool built in Rust with a React/Vite web UI. It parses OpenAPI 3.x and Arazzo Specification files to visualize API workflows, convert them to test scripts (k6), and execute tests.

### Core Philosophy
- **OpenAPI/Arazzo as Source of Truth**: Writing API documentation doubles as writing tests
- **Graph-Based Workflow**: API calls are modeled as directed graphs with data dependencies
- **Phased Implementation**: Phase 1 (visualization) → Phase 2 (external tool integration) → Phase 3 (native Rust engine)

## Architecture

### Module Structure

```
src/
├── loader/           # Parse and load OpenAPI/Arazzo YAML files
│   ├── openapi.rs           # OpenAPI parsing using oas3 crate
│   ├── arazzo.rs            # Custom Arazzo parser
│   ├── openapi_resolver.rs # Resolve $ref and operationId/operationPath
│   └── project.rs           # Multi-project mode: scan directories
├── models/           # Data structures
│   └── arazzo.rs            # Arazzo spec models (Workflow, Step, etc.)
├── graph/            # Flow graph construction and export
│   ├── builder.rs           # Build petgraph DiGraph from Arazzo workflows
│   ├── validator.rs         # Validate graph structure and references
│   └── exporter.rs          # Export to DOT/JSON/Mermaid
├── converters/       # Convert workflows to external formats
│   └── k6.rs                # Generate k6 JavaScript test scripts
├── runner/           # Execute tests with external engines
│   └── k6.rs                # Run k6 and parse results
├── server/           # Web server (Axum)
│   ├── api.rs               # API endpoints for multi-project mode
│   └── state.rs             # Shared application state
├── commands/         # CLI command implementations
│   ├── list.rs              # List workflows
│   ├── validate.rs          # Validate specs
│   ├── visualize.rs         # Generate flow graphs
│   ├── convert.rs           # Convert to k6 scripts
│   ├── serve.rs             # Start web server
│   └── export_*.rs          # Export hornet2's own OpenAPI/Arazzo
└── cli.rs            # CLI argument parsing (clap)

ui/
├── src/
│   ├── components/   # React components
│   │   ├── CytoscapeView.tsx     # Graph visualization (cytoscape.js)
│   │   ├── OperationList.tsx     # Browse OpenAPI operations
│   │   ├── WorkflowView.tsx      # Visual workflow editor
│   │   └── YamlEditor.tsx        # Monaco editor integration
│   └── pages/
│       ├── VisualizationPage.tsx # View existing workflows
│       └── EditorPage.tsx        # Create/edit workflows
```

### Key Data Flow

1. **Loading**: `loader` modules parse YAML → `models::arazzo` structures
2. **Resolution**: `OpenApiResolver` resolves operationId/operationPath to actual OpenAPI operations
3. **Graph Building**: `graph::builder` converts Arazzo steps into `FlowGraph` (petgraph DiGraph)
   - Nodes: `FlowNode` (step_id, operation_id, method, etc.)
   - Edges: `FlowEdge` (Sequential, Conditional, DataDependency)
   - Data dependency detection: regex parsing of `$steps.{step_id}.outputs.*` references
4. **Export/Convert**: `exporter` outputs DOT/JSON/Mermaid, `converters::k6` generates JavaScript
5. **Execution**: `runner::k6` runs generated scripts via external k6 binary

### Graph Construction Details

The `graph::builder` module is central to the architecture:

- **Sequential Edges**: Steps without explicit dependencies are connected sequentially
- **Data Dependency Edges**: Detected by parsing `$steps.{step_id}.outputs.*` in request bodies, parameters, and headers
- **Conditional Edges**: Created from `successCriteria` and `onSuccess`/`onFailure` paths
- **Step Resolution**: `operationId` and `operationPath` + `method` are resolved to OpenAPI operations to extract HTTP method and path

## Common Commands

### Development Workflow

**Tool Requirements**: Install via `mise install` (requires [mise](https://mise.jdx.dev/))
- Rust: stable
- Node.js: 22
- pnpm: 10

**Quick Start**:
```bash
# Install all dependencies
make install

# Development mode (both Rust server + UI dev server)
make dev
# → CLI Server: http://localhost:3000
# → UI Dev Server: http://localhost:5173 (open this in browser)
```

### Building

```bash
make build           # Build both CLI and UI
make cli-build       # cargo build --release
make ui-build        # cd ui && pnpm build
```

### Testing

```bash
make test            # Run all tests
make cli-test        # cargo test
make ui-test         # cd ui && pnpm test:run
make ui-test-watch   # cd ui && pnpm test
```

### Code Quality

```bash
make check           # Format check + clippy + UI lint/typecheck
make fmt             # Format Rust and UI code
make lint            # Run UI lint

# Individual checks
cargo fmt --check
cargo clippy -- -D warnings
cd ui && pnpm typecheck
cd ui && pnpm lint
```

### Running Hornet2

**Serve mode (multi-project)**:
```bash
# Uses hornet2's own OpenAPI/Arazzo in root directory
make cli-dev
cargo run -- serve --root-dir . --port 3000
```

**Legacy single-file mode**:
```bash
# Validate specs
cargo run -- validate --openapi tests/fixtures/openapi.yaml --arazzo tests/fixtures/arazzo.yaml

# Visualize workflow
cargo run -- visualize --arazzo tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml --format json
cargo run -- visualize --arazzo tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml --format dot
make visualize ARGS="--format json"

# Convert to k6 script
cargo run -- convert --arazzo tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml --to k6

# Run tests with k6 (requires k6 installed)
cargo run -- run --arazzo tests/fixtures/arazzo.yaml --openapi tests/fixtures/openapi.yaml --engine k6
```

### Running Tests for a Single Module

```bash
# Run tests for a specific module
cargo test --test integration_test
cargo test graph::builder::tests
cargo test --lib graph

# Run UI tests for a specific component
cd ui && pnpm test OperationList
```

## Key Concepts

### Arazzo Specification

Arazzo extends OpenAPI by defining **workflows** (sequences of API calls with data passing between steps):

```yaml
workflows:
  - workflowId: user-onboarding-flow
    steps:
      - stepId: register
        operationId: registerUser
        requestBody:
          payload:
            username: $inputs.username

      - stepId: login
        operationId: loginUser
        requestBody:
          payload:
            username: $inputs.username
            password: $inputs.password
        outputs:
          token: $response.body.token

      - stepId: getProfile
        operationId: getUserProfile
        parameters:
          - name: Authorization
            in: header
            value: Bearer $steps.login.outputs.token  # Data dependency
```

**Runtime References**:
- `$inputs.*`: Workflow inputs
- `$steps.{stepId}.outputs.*`: Outputs from previous steps
- `$response.body.*`: Response body fields
- `$statusCode`: HTTP status code

### Multi-Project Mode

The server supports managing multiple projects in a root directory:

```
root_dir/
├── project-a/
│   ├── openapi.yaml
│   └── arazzo.yaml
└── project-b/
    ├── openapi.yaml
    └── arazzo.yaml
```

API endpoints:
- `GET /api/projects` - List all projects
- `GET /api/projects/{name}` - Get project details
- `GET /api/projects/{name}/workflows` - List workflows
- `GET /api/projects/{name}/graph/{workflow_id}` - Get graph visualization

### Single-Project Mode (hornet2 itself)

Hornet2 can also run in single-project mode where OpenAPI and Arazzo files are in the root directory:

```bash
# Use hornet2's own specs
cargo run -- serve --root-dir . --port 3000
```

This mode is used for developing hornet2 itself using `make dev`.

## Development Guidelines

### Working with Graphs

When modifying graph building logic (`graph/builder.rs`):

1. Understand the three edge types:
   - **Sequential**: Default connection between steps
   - **DataDependency**: Created when `$steps.{id}.outputs.*` is detected
   - **Conditional**: Created from `successCriteria` and failure paths

2. Data dependency detection uses regex patterns to find runtime references in:
   - Request body payloads (JSON)
   - Request parameters (path, query, header)
   - Success criteria conditions

3. Test with `tests/fixtures/arazzo.yaml` which contains realistic workflows

### Adding New Export Formats

To add a new export format (e.g., PlantUML):

1. Add variant to `cli::OutputFormat` enum
2. Implement export logic in `graph/exporter.rs`
3. Add tests using existing test fixtures
4. Update CLI help text

### Adding New Test Engines

To support engines beyond k6:

1. Create new module in `converters/` (e.g., `converters/jmeter.rs`)
2. Implement script generation from `FlowGraph`
3. Create runner in `runner/` (e.g., `runner/jmeter.rs`)
4. Add CLI option in `cli.rs` and command in `commands/convert.rs`

### UI Development

The UI uses:
- **Zustand**: State management
- **Cytoscape.js**: Graph rendering with interactive layouts
- **Monaco Editor**: YAML editing with validation
- **React 19**: UI framework

To add new UI features:
1. Add API endpoint in `src/server/api.rs`
2. Create component in `ui/src/components/`
3. Add to appropriate page in `ui/src/pages/`
4. Test with Vitest (`ui/src/components/__tests__/`)

## Testing Strategy

### Rust Tests

- **Unit tests**: Co-located in source files (`#[cfg(test)]`)
- **Integration tests**: `tests/integration_test.rs`
- **Fixtures**: `tests/fixtures/` contains sample OpenAPI/Arazzo files

### UI Tests

- **Component tests**: `ui/src/components/__tests__/`
- **Testing Library**: React Testing Library + Vitest
- **Run**: `make ui-test` or `make ui-test-watch`

### Manual Testing

Use `make dev` to test full-stack changes:
1. Make changes to Rust backend
2. Make changes to UI
3. Both reload automatically (UI hot-reloads, Rust requires restart)
4. Test via browser at http://localhost:5173

## Debugging

### Rust Server Logs

The server uses `tracing` for logs. To see debug logs:

```bash
RUST_LOG=debug cargo run -- serve --root-dir . --port 3000
```

### UI Development

Open browser DevTools at http://localhost:5173. The UI makes API calls to http://localhost:3000 via Vite proxy.

### Graph Visualization Issues

If graphs don't render correctly:
1. Check `/api/projects/{name}/graph/{workflow_id}` endpoint response
2. Verify `FlowGraph` has correct nodes/edges in `graph/builder.rs`
3. Check browser console for Cytoscape.js errors
4. Test with `cargo run -- visualize ... --format json` to see raw graph data

## Repository Patterns

### Error Handling

- Use `thiserror` for custom error types (`src/error.rs`)
- Commands return `Result<()>` where `Result = std::result::Result<T, HornetError>`
- Propagate errors with `?` operator

### Code Style

- Rust: Follow `rustfmt` defaults (`cargo fmt`)
- UI: Follow Prettier/ESLint config (`make fmt`, `make lint`)
- Module organization: Re-export public APIs in `mod.rs`

### Commit Messages

Follow conventional commits format (already in use based on git history):
```
feat: add new export format
fix: resolve data dependency detection bug
docs: update README with new examples
```
