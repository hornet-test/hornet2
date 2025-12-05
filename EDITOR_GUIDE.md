# Arazzo Web Editor - User Guide

## Overview

The Arazzo Web Editor is a visual tool for creating and editing Arazzo workflows. It provides:

- **Operation List**: Browse and filter OpenAPI operations
- **Visual Workflow View**: See your workflow steps in a clear, visual format
- **YAML Editor**: Edit Arazzo YAML with syntax highlighting and real-time validation
- **Live Sync**: Changes in the visual editor sync to YAML and vice versa

## Getting Started

### 1. Start the Development Server

```bash
make dev
```

This will start:
- Backend API server on http://localhost:3000
- Frontend dev server on http://localhost:5173

### 2. Open the Editor

Navigate to http://localhost:5173 and click the **Editor** tab.

## Using the Editor

### Operations Sidebar

The left sidebar shows all operations from your OpenAPI spec.

**Features**:
- **Search**: Filter operations by name, path, or summary
- **Method Filters**: Click method badges (GET, POST, etc.) to filter by HTTP method
- **Add to Workflow**: Hover over an operation and click the "+ Add" button

### Visual Workflow View

The center panel shows your workflow steps visually.

**Features**:
- See all steps in sequence with numbered indicators
- View operation IDs, parameters, and success criteria
- Empty state with helpful prompt when no steps exist

### YAML Editor

The right panel provides a powerful code editor.

**Features**:
- Syntax highlighting for YAML
- Real-time validation with error markers
- Line/column error reporting
- Auto-formatting

### View Modes

Toggle between three view modes using the toolbar:

- **Visual**: Show only the workflow view
- **Split**: Show both workflow and YAML side-by-side (default)
- **YAML**: Show only the YAML editor

## Workflow

### Creating a Workflow

1. Start with an empty workflow
2. Add operations from the sidebar by clicking "+ Add"
3. Steps are automatically added to the first workflow
4. View the generated YAML in real-time

### Editing in YAML

1. Click in the YAML editor
2. Make changes to the YAML
3. Validation runs automatically after 500ms
4. Errors appear as red underlines with hover tooltips
5. Visual view updates to reflect changes

### Validation

The editor validates your Arazzo spec in real-time:

- **Green checkmark**: Valid spec
- **Red X with count**: Invalid with error count
- **Error panel**: Shows detailed error messages
- **Editor markers**: Red underlines at error locations

## API Endpoints

The editor uses these backend APIs:

### GET /api/editor/operations

Returns all OpenAPI operations.

**Response**:
```json
{
  "operations": [
    {
      "operation_id": "registerUser",
      "method": "POST",
      "path": "/register",
      "summary": "Register a new user",
      "description": "...",
      "parameters": [...],
      "request_body": {...},
      "tags": ["users"]
    }
  ]
}
```

### POST /api/editor/validate

Validates Arazzo YAML.

**Request**:
```json
{
  "yaml": "arazzo: 1.0.0\n..."
}
```

**Response**:
```json
{
  "valid": true,
  "errors": []
}
```

Or with errors:
```json
{
  "valid": false,
  "errors": [
    {
      "message": "At least one workflow is required",
      "line": null,
      "column": null
    }
  ]
}
```

## Keyboard Shortcuts

In the YAML editor:
- `Cmd/Ctrl + F`: Find
- `Cmd/Ctrl + H`: Find and replace
- `Cmd/Ctrl + Z`: Undo
- `Cmd/Ctrl + Shift + Z`: Redo
- `Tab`: Indent
- `Shift + Tab`: Outdent

## Tips

1. **Start Simple**: Add one operation, see the YAML, then build from there
2. **Use Search**: Quickly find operations by typing in the search box
3. **Method Filters**: Filter by HTTP method to find related operations
4. **Split View**: Use split view to see visual and YAML simultaneously
5. **Validation**: Wait for the green checkmark before saving or exporting

## Current Limitations

This is Phase 1 MVP. Future enhancements will include:

- Drag-and-drop workflow building
- Data flow suggestions between steps
- OAS links detection and workflow suggestions
- Template library
- Export to k6/Postman
- AI-powered workflow generation

## Troubleshooting

### "No operations available"

- Ensure OpenAPI file is loaded correctly
- Check that the OpenAPI file has valid paths
- Restart the server with a valid OpenAPI file

### Validation errors not showing

- Wait 500ms after typing for validation to run
- Check browser console for errors
- Ensure backend API is running

### YAML not syncing to visual view

- Check for YAML syntax errors
- Ensure the spec is valid Arazzo format
- Look for validation errors in the error panel

## Next Steps

After creating your workflow:

1. Switch to the **Visualization** tab to see the flow graph
2. Save the YAML to a file (copy from editor)
3. Run the workflow with `cargo run -- list --arazzo <file>`
4. Validate with `cargo run -- validate --arazzo <file> --openapi <openapi>`
