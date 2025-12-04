import React, { useEffect, useState } from 'react';
import { Controls } from './components/Controls.jsx';
import { CytoscapeView } from './components/CytoscapeView.jsx';
import { DetailsPanel } from './components/DetailsPanel.jsx';
import { StatusBar } from './components/StatusBar.jsx';

const layoutOptions = [
  { value: 'dagre', label: 'Hierarchical (Dagre)' },
  { value: 'breadthfirst', label: 'Breadth First' },
  { value: 'circle', label: 'Circle' },
  { value: 'grid', label: 'Grid' },
];

export default function App() {
  const [workflows, setWorkflows] = useState([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState('');
  const [layout, setLayout] = useState('dagre');
  const [status, setStatus] = useState({ message: 'Loading workflows...', type: 'info' });
  const [graph, setGraph] = useState(null);
  const [selectedNode, setSelectedNode] = useState(null);

  const fetchWorkflows = async () => {
    try {
      setStatus({ message: 'Loading workflows...', type: 'info' });
      const response = await fetch('/api/workflows');
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const data = await response.json();
      const newWorkflows = data.workflows || [];
      setWorkflows(newWorkflows);

      if (newWorkflows.length) {
        setSelectedWorkflow(newWorkflows[0].workflow_id);
      } else {
        setSelectedWorkflow('');
        setGraph(null);
        setStatus({ message: 'No workflows found. Please load an Arazzo file.', type: 'error' });
        return;
      }

      setStatus({ message: `Loaded ${newWorkflows.length} workflow(s)`, type: 'success' });
    } catch (error) {
      setStatus({ message: `Error: ${error.message}`, type: 'error' });
    }
  };

  const fetchGraph = async (workflowId) => {
    if (!workflowId) return;
    try {
      setStatus({ message: `Loading graph for ${workflowId}...`, type: 'info' });
      const response = await fetch(`/api/graph/${encodeURIComponent(workflowId)}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const data = await response.json();
      setGraph(data);
      setStatus({ message: `Rendered ${data.nodes.length} nodes and ${data.edges.length} edges`, type: 'success' });
    } catch (error) {
      setStatus({ message: `Error: ${error.message}`, type: 'error' });
    }
  };

  useEffect(() => {
    fetchWorkflows();
  }, []);

  useEffect(() => {
    if (selectedWorkflow) {
      fetchGraph(selectedWorkflow);
    }
  }, [selectedWorkflow]);

  const handleWorkflowChange = (event) => {
    setSelectedWorkflow(event.target.value);
    setSelectedNode(null);
  };

  const handleLayoutChange = (event) => {
    setLayout(event.target.value);
  };

  const handleRefresh = () => {
    setSelectedNode(null);
    fetchWorkflows();
  };

  return (
    <div className="container">
      <header>
        <h1>Hornet2 - API Flow Visualization</h1>
        <p className="subtitle">Document-driven API testing tool</p>
      </header>

      <Controls
        workflows={workflows}
        selectedWorkflow={selectedWorkflow}
        layout={layout}
        onWorkflowChange={handleWorkflowChange}
        onLayoutChange={handleLayoutChange}
        onRefresh={handleRefresh}
        layoutOptions={layoutOptions}
      />

      <div className="main-content">
        <CytoscapeView
          graph={graph}
          layout={layout}
          onNodeSelected={(data) => setSelectedNode(data)}
          onCanvasCleared={() => setSelectedNode(null)}
        />
        <DetailsPanel nodeData={selectedNode} />
      </div>

      <StatusBar message={status.message} type={status.type} />
    </div>
  );
}
