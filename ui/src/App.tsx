import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Controls } from './components/Controls';
import { CytoscapeView } from './components/CytoscapeView';
import { DetailsPanel } from './components/DetailsPanel';
import { StatusBar } from './components/StatusBar';
import type {
  GraphData,
  LayoutOption,
  SelectedNode,
  StatusMessage,
  WorkflowSummary,
} from './types/graph';

const layoutOptions: LayoutOption[] = [
  { value: 'dagre', label: 'Hierarchical (Dagre)' },
  { value: 'breadthfirst', label: 'Breadth First' },
  { value: 'circle', label: 'Circle' },
  { value: 'grid', label: 'Grid' },
];

export default function App() {
  const [workflows, setWorkflows] = useState<WorkflowSummary[]>([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState<string>('');
  const [layout, setLayout] = useState<LayoutOption['value']>('dagre');
  const [status, setStatus] = useState<StatusMessage>({
    message: 'Loading workflows...',
    type: 'info',
  });
  const [graph, setGraph] = useState<GraphData | null>(null);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);

  const selectedWorkflowLabel = useMemo(
    () => workflows.find((wf) => wf.workflow_id === selectedWorkflow)?.workflow_id ?? '',
    [selectedWorkflow, workflows],
  );

  const fetchWorkflows = useCallback(async () => {
    try {
      setStatus({ message: 'Loading workflows...', type: 'info' });
      const response = await fetch('/api/workflows');
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const data = (await response.json()) as { workflows?: WorkflowSummary[] };
      const newWorkflows = data.workflows ?? [];
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
      setStatus({ message: `Error: ${(error as Error).message}`, type: 'error' });
    }
  }, []);

  const fetchGraph = useCallback(async (workflowId: string) => {
    if (!workflowId) return;
    try {
      setStatus({ message: `Loading graph for ${workflowId}...`, type: 'info' });
      const response = await fetch(`/api/graph/${encodeURIComponent(workflowId)}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const data = (await response.json()) as GraphData;
      setGraph(data);
      setStatus({
        message: `Rendered ${data.nodes.length} nodes and ${data.edges.length} edges`,
        type: 'success',
      });
    } catch (error) {
      setStatus({ message: `Error: ${(error as Error).message}`, type: 'error' });
    }
  }, []);

  useEffect(() => {
    void fetchWorkflows();
  }, [fetchWorkflows]);

  useEffect(() => {
    if (selectedWorkflow) {
      void fetchGraph(selectedWorkflow);
    }
  }, [fetchGraph, selectedWorkflow]);

  const handleWorkflowChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedWorkflow(event.target.value);
    setSelectedNode(null);
  };

  const handleLayoutChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setLayout(event.target.value as LayoutOption['value']);
  };

  const handleRefresh = () => {
    setSelectedNode(null);
    void fetchWorkflows();
  };

  const handleNodeSelected = useCallback((data: SelectedNode) => {
    setSelectedNode(data);
  }, []);

  const handleCanvasCleared = useCallback(() => {
    setSelectedNode(null);
  }, []);

  return (
    <div className="container">
      <header>
        <h1>Hornet2 - API Flow Visualization</h1>
        <p className="subtitle">Document-driven API testing tool</p>
      </header>

      <Controls
        workflows={workflows}
        selectedWorkflow={selectedWorkflowLabel}
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
          onNodeSelected={handleNodeSelected}
          onCanvasCleared={handleCanvasCleared}
        />
        <DetailsPanel nodeData={selectedNode} />
      </div>

      <StatusBar message={status.message} type={status.type} />
    </div>
  );
}
