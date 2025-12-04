import React, { useEffect, useRef } from 'react';
import { createCytoscape } from '../lib/cytoscape.js';

const styles = [
  {
    selector: 'node',
    style: {
      'background-color': '#667eea',
      label: 'data(label)',
      color: '#333',
      'text-valign': 'center',
      'text-halign': 'center',
      'font-size': '12px',
      'font-weight': 'bold',
      width: '80px',
      height: '80px',
      'text-wrap': 'wrap',
      'text-max-width': '70px',
      'border-width': '2px',
      'border-color': '#5568d3',
    },
  },
  {
    selector: 'node[method="GET"]',
    style: { 'background-color': '#61affe' },
  },
  {
    selector: 'node[method="POST"]',
    style: { 'background-color': '#49cc90' },
  },
  {
    selector: 'node[method="PUT"]',
    style: { 'background-color': '#fca130' },
  },
  {
    selector: 'node[method="DELETE"]',
    style: { 'background-color': '#f93e3e' },
  },
  {
    selector: 'node[method="PATCH"]',
    style: { 'background-color': '#50e3c2' },
  },
  {
    selector: 'edge',
    style: {
      width: 2,
      'line-color': '#999',
      'target-arrow-color': '#999',
      'target-arrow-shape': 'triangle',
      'curve-style': 'bezier',
      'arrow-scale': 1.5,
    },
  },
  {
    selector: 'edge[edge_type="Sequential"]',
    style: {
      'line-color': '#4caf50',
      'target-arrow-color': '#4caf50',
      'line-style': 'solid',
    },
  },
  {
    selector: 'edge[edge_type="Conditional"]',
    style: {
      'line-color': '#ff9800',
      'target-arrow-color': '#ff9800',
      'line-style': 'dashed',
    },
  },
  {
    selector: 'edge[edge_type="DataDependency"]',
    style: {
      'line-color': '#2196f3',
      'target-arrow-color': '#2196f3',
      'line-style': 'dotted',
    },
  },
  {
    selector: ':selected',
    style: {
      'border-width': '4px',
      'border-color': '#764ba2',
      'background-color': '#764ba2',
    },
  },
];

function applyLayout(cy, layoutName) {
  if (!cy) return;
  let options = {
    name: layoutName,
    animate: true,
    animationDuration: 500,
  };

  if (layoutName === 'dagre') {
    options = {
      ...options,
      rankDir: 'TB',
      nodeSep: 50,
      rankSep: 100,
    };
  } else if (layoutName === 'breadthfirst') {
    options = {
      ...options,
      directed: true,
      spacingFactor: 1.5,
    };
  }

  cy.layout(options).run();
  setTimeout(() => cy.fit(null, 50), 600);
}

export function CytoscapeView({ graph, layout, onNodeSelected, onCanvasCleared }) {
  const containerRef = useRef(null);
  const cyRef = useRef(null);

  useEffect(() => {
    if (!containerRef.current) return undefined;
    const cy = createCytoscape({
      container: containerRef.current,
      style: styles,
      elements: [],
      layout: { name: 'dagre', rankDir: 'TB', nodeSep: 50, rankSep: 100 },
      minZoom: 0.3,
      maxZoom: 3,
      wheelSensitivity: 0.2,
    });

    cy.on('tap', 'node', (evt) => {
      const node = evt.target;
      onNodeSelected({ ...node.data(), connectedEdges: node.connectedEdges().length });
    });

    cy.on('tap', (evt) => {
      if (evt.target === cy) {
        onCanvasCleared();
      }
    });

    cyRef.current = cy;

    return () => cy.destroy();
  }, [onCanvasCleared, onNodeSelected]);

  useEffect(() => {
    if (!cyRef.current || !graph) return;
    const cy = cyRef.current;
    const elements = [];

    graph.nodes.forEach((node) => {
      elements.push({
        data: {
          id: node.id,
          label: node.label || node.id,
          method: node.method,
          operation_id: node.operation_id,
          path: node.path,
          description: node.description,
        },
      });
    });

    graph.edges.forEach((edge) => {
      elements.push({
        data: {
          id: `${edge.source}-${edge.target}`,
          source: edge.source,
          target: edge.target,
          edge_type: edge.edge_type,
          label: edge.label,
        },
      });
    });

    cy.elements().remove();
    cy.add(elements);
    applyLayout(cy, layout);
  }, [graph, layout]);

  useEffect(() => {
    if (cyRef.current) {
      applyLayout(cyRef.current, layout);
    }
  }, [layout]);

  return <div id="cy" ref={containerRef} data-testid="cytoscape-container" />;
}
