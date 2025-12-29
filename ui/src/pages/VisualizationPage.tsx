import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Controls } from '../components/Controls';
import { CytoscapeView } from '../components/CytoscapeView';
import { DetailsPanel } from '../components/DetailsPanel';
import { StatusBar } from '../components/StatusBar';
import type {
  GraphData,
  LayoutOption,
  SelectedNode,
  StatusMessage,
  WorkflowSummary,
} from '../types/graph';
import { useProjectStore } from '../stores/projectStore';

const layoutOptions: LayoutOption[] = [
  { value: 'dagre', label: 'Hierarchical (Dagre)' },
  { value: 'breadthfirst', label: 'Breadth First' },
  { value: 'circle', label: 'Circle' },
  { value: 'grid', label: 'Grid' },
];

export const VisualizationPage: React.FC = () => {
  const { currentProject: project } = useProjectStore();
  const [workflows, setWorkflows] = useState<WorkflowSummary[]>([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState<string>('');
  const [layout, setLayout] = useState<LayoutOption['value']>('dagre');
  const [status, setStatus] = useState<StatusMessage>({
    message: 'Loading projects...',
    type: 'info',
  });
  const [graph, setGraph] = useState<GraphData | null>(null);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);

  const selectedWorkflowLabel = useMemo(
    () => workflows.find((wf) => wf.workflow_id === selectedWorkflow)?.workflow_id ?? '',
    [selectedWorkflow, workflows],
  );

  // Removed fetchProjects and replaced with store usage is implicit by removing the definition

  const fetchWorkflows = useCallback(async () => {
    if (!project) return;
    try {
      setStatus({ message: 'Loading workflows...', type: 'info' });
      const response = await fetch(`/api/projects/${project}/workflows`);
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const data = (await response.json()) as { workflows?: WorkflowSummary[] };
      const newWorkflows = data.workflows ?? [];
      setWorkflows(newWorkflows);

      if (newWorkflows.length) {
        setSelectedWorkflow(newWorkflows[0].workflow_id);
      } else {
        setSelectedWorkflow('');
        setGraph(null);
        setStatus({ message: `No workflows found for project ${project}`, type: 'error' });
        return;
      }

      setStatus({ message: `Loaded ${newWorkflows.length} workflow(s)`, type: 'success' });
    } catch (error) {
      setStatus({ message: `Error: ${(error as Error).message}`, type: 'error' });
    }
  }, [project]);

  const fetchGraph = useCallback(
    async (workflowId: string) => {
      if (!workflowId || !project) return;
      try {
        setStatus({ message: `Loading graph for ${workflowId}...`, type: 'info' });
        const response = await fetch(
          `/api/projects/${project}/graph/${encodeURIComponent(workflowId)}`,
        );
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
    },
    [project],
  );

  // Removed fetchProjects effect

  useEffect(() => {
    if (project) {
      void fetchWorkflows();
    }
  }, [project, fetchWorkflows]);

  useEffect(() => {
    if (selectedWorkflow && project) {
      void fetchGraph(selectedWorkflow);
    }
  }, [fetchGraph, selectedWorkflow, project]);

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
    <div className="visualization-page">
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

      <style>{`
        .visualization-page {
          display: flex;
          flex-direction: column;
          height: 100%;
        }

        .main-content {
          flex: 1;
          display: flex;
          overflow: hidden;
        }
      `}</style>
    </div>
  );
};
