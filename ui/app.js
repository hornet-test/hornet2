// Hornet2 - API Flow Visualization

let cy = null;
let currentWorkflows = [];
let currentGraph = null;

// Initialize the application
document.addEventListener('DOMContentLoaded', async () => {
    initCytoscape();
    setupEventListeners();
    await loadWorkflows();
});

// Initialize Cytoscape
function initCytoscape() {
    cy = cytoscape({
        container: document.getElementById('cy'),
        style: [
            {
                selector: 'node',
                style: {
                    'background-color': '#667eea',
                    'label': 'data(label)',
                    'color': '#333',
                    'text-valign': 'center',
                    'text-halign': 'center',
                    'font-size': '12px',
                    'font-weight': 'bold',
                    'width': '80px',
                    'height': '80px',
                    'text-wrap': 'wrap',
                    'text-max-width': '70px',
                    'border-width': '2px',
                    'border-color': '#5568d3'
                }
            },
            {
                selector: 'node[method="GET"]',
                style: {
                    'background-color': '#61affe'
                }
            },
            {
                selector: 'node[method="POST"]',
                style: {
                    'background-color': '#49cc90'
                }
            },
            {
                selector: 'node[method="PUT"]',
                style: {
                    'background-color': '#fca130'
                }
            },
            {
                selector: 'node[method="DELETE"]',
                style: {
                    'background-color': '#f93e3e'
                }
            },
            {
                selector: 'node[method="PATCH"]',
                style: {
                    'background-color': '#50e3c2'
                }
            },
            {
                selector: 'edge',
                style: {
                    'width': 2,
                    'line-color': '#999',
                    'target-arrow-color': '#999',
                    'target-arrow-shape': 'triangle',
                    'curve-style': 'bezier',
                    'arrow-scale': 1.5
                }
            },
            {
                selector: 'edge[edge_type="Sequential"]',
                style: {
                    'line-color': '#4caf50',
                    'target-arrow-color': '#4caf50',
                    'line-style': 'solid'
                }
            },
            {
                selector: 'edge[edge_type="Conditional"]',
                style: {
                    'line-color': '#ff9800',
                    'target-arrow-color': '#ff9800',
                    'line-style': 'dashed'
                }
            },
            {
                selector: 'edge[edge_type="DataDependency"]',
                style: {
                    'line-color': '#2196f3',
                    'target-arrow-color': '#2196f3',
                    'line-style': 'dotted'
                }
            },
            {
                selector: ':selected',
                style: {
                    'border-width': '4px',
                    'border-color': '#764ba2',
                    'background-color': '#764ba2'
                }
            }
        ],
        elements: [],
        layout: {
            name: 'dagre',
            rankDir: 'TB',
            nodeSep: 50,
            rankSep: 100
        },
        minZoom: 0.3,
        maxZoom: 3,
        wheelSensitivity: 0.2
    });

    // Node click handler
    cy.on('tap', 'node', function(evt) {
        const node = evt.target;
        showNodeDetails(node);
    });

    // Click on background to clear selection
    cy.on('tap', function(evt) {
        if (evt.target === cy) {
            clearNodeDetails();
        }
    });
}

// Setup event listeners
function setupEventListeners() {
    document.getElementById('workflow-select').addEventListener('change', onWorkflowChange);
    document.getElementById('layout-select').addEventListener('change', onLayoutChange);
    document.getElementById('refresh-btn').addEventListener('click', loadWorkflows);
}

// Load workflows from API
async function loadWorkflows() {
    try {
        setStatus('Loading workflows...', 'info');
        const response = await fetch('/api/workflows');

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        currentWorkflows = data.workflows;

        const select = document.getElementById('workflow-select');
        select.innerHTML = '';

        if (currentWorkflows.length === 0) {
            select.innerHTML = '<option value="">No workflows found</option>';
            setStatus('No workflows found. Please load an Arazzo file.', 'error');
            return;
        }

        currentWorkflows.forEach(workflow => {
            const option = document.createElement('option');
            option.value = workflow.workflow_id;
            option.textContent = `${workflow.workflow_id} (${workflow.steps} steps)`;
            select.appendChild(option);
        });

        setStatus(`Loaded ${currentWorkflows.length} workflow(s)`, 'success');

        // Load the first workflow by default
        if (currentWorkflows.length > 0) {
            await loadGraph(currentWorkflows[0].workflow_id);
        }
    } catch (error) {
        console.error('Failed to load workflows:', error);
        setStatus(`Error: ${error.message}`, 'error');
    }
}

// Load graph for a specific workflow
async function loadGraph(workflowId) {
    try {
        setStatus(`Loading graph for ${workflowId}...`, 'info');
        const response = await fetch(`/api/graph/${encodeURIComponent(workflowId)}`);

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const graph = await response.json();
        currentGraph = graph;

        renderGraph(graph);
        setStatus(`Rendered ${graph.nodes.length} nodes and ${graph.edges.length} edges`, 'success');
    } catch (error) {
        console.error('Failed to load graph:', error);
        setStatus(`Error: ${error.message}`, 'error');
    }
}

// Render graph using Cytoscape
function renderGraph(graph) {
    const elements = [];

    // Add nodes
    graph.nodes.forEach(node => {
        elements.push({
            data: {
                id: node.id,
                label: node.label || node.id,
                method: node.method,
                operation_id: node.operation_id,
                path: node.path,
                description: node.description
            }
        });
    });

    // Add edges
    graph.edges.forEach(edge => {
        elements.push({
            data: {
                id: `${edge.source}-${edge.target}`,
                source: edge.source,
                target: edge.target,
                edge_type: edge.edge_type,
                label: edge.label
            }
        });
    });

    cy.elements().remove();
    cy.add(elements);

    // Apply layout
    const layoutName = document.getElementById('layout-select').value;
    applyLayout(layoutName);
}

// Apply layout
function applyLayout(layoutName) {
    let layoutOptions = {
        name: layoutName,
        animate: true,
        animationDuration: 500
    };

    if (layoutName === 'dagre') {
        layoutOptions = {
            ...layoutOptions,
            rankDir: 'TB',
            nodeSep: 50,
            rankSep: 100
        };
    } else if (layoutName === 'breadthfirst') {
        layoutOptions = {
            ...layoutOptions,
            directed: true,
            spacingFactor: 1.5
        };
    }

    cy.layout(layoutOptions).run();

    // Fit to viewport
    setTimeout(() => {
        cy.fit(null, 50);
    }, 600);
}

// Show node details in side panel
function showNodeDetails(node) {
    const data = node.data();
    const detailsContent = document.getElementById('details-content');

    let html = '';

    // Node ID
    html += `
        <div class="detail-item">
            <div class="detail-label">Step ID</div>
            <div class="detail-value"><code>${escapeHtml(data.id)}</code></div>
        </div>
    `;

    // HTTP Method
    if (data.method) {
        html += `
            <div class="detail-item">
                <div class="detail-label">HTTP Method</div>
                <div class="detail-value">
                    <span class="http-method ${data.method.toLowerCase()}">${data.method}</span>
                </div>
            </div>
        `;
    }

    // Operation ID
    if (data.operation_id) {
        html += `
            <div class="detail-item">
                <div class="detail-label">Operation ID</div>
                <div class="detail-value"><code>${escapeHtml(data.operation_id)}</code></div>
            </div>
        `;
    }

    // Path
    if (data.path) {
        html += `
            <div class="detail-item">
                <div class="detail-label">Path</div>
                <div class="detail-value"><code>${escapeHtml(data.path)}</code></div>
            </div>
        `;
    }

    // Description
    if (data.description) {
        html += `
            <div class="detail-item">
                <div class="detail-label">Description</div>
                <div class="detail-value">${escapeHtml(data.description)}</div>
            </div>
        `;
    }

    // Connected edges
    const connectedEdges = node.connectedEdges();
    if (connectedEdges.length > 0) {
        html += `
            <div class="detail-item">
                <div class="detail-label">Connections</div>
                <div class="detail-value">
                    ${connectedEdges.length} edge(s)
                </div>
            </div>
        `;
    }

    detailsContent.innerHTML = html;
}

// Clear node details
function clearNodeDetails() {
    const detailsContent = document.getElementById('details-content');
    detailsContent.innerHTML = '<p class="placeholder">Click on a node to see details</p>';
}

// Event handler for workflow selection change
async function onWorkflowChange(event) {
    const workflowId = event.target.value;
    if (workflowId) {
        await loadGraph(workflowId);
    }
}

// Event handler for layout change
function onLayoutChange(event) {
    const layoutName = event.target.value;
    if (currentGraph) {
        applyLayout(layoutName);
    }
}

// Set status message
function setStatus(message, type = 'info') {
    const statusEl = document.getElementById('status');
    statusEl.textContent = message;
    statusEl.className = `status ${type}`;
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
