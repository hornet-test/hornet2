import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Controls } from '../components/Controls';
import { CytoscapeView } from '../components/CytoscapeView';
import { DetailsPanel } from '../components/DetailsPanel';
import { StatusBar } from '../components/StatusBar';
import ApiDocDrawer from '../components/ApiDocDrawer';
import type {
  GraphData,
  LayoutOption,
  SelectedNode,
  StatusMessage,
  WorkflowSummary,
} from '../types/graph';
import { useProjectStore } from '../stores/projectStore';
import { useEditorStore } from '../stores/editorStore';

const layoutOptions: LayoutOption[] = [
  { value: 'dagre', label: 'Hierarchical (Dagre)' },
  { value: 'breadthfirst', label: 'Breadth First' },
  { value: 'circle', label: 'Circle' },
  { value: 'grid', label: 'Grid' },
];

export const VisualizationPage: React.FC = () => {
  const { currentProject: project } = useProjectStore();
  const [searchParams, setSearchParams] = useSearchParams();
  const [workflows, setWorkflows] = useState<WorkflowSummary[]>([]);
  const [status, setStatus] = useState<StatusMessage>({
    message: 'Loading projects...',
    type: 'info',
  });
  const [graph, setGraph] = useState<GraphData | null>(null);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);

  // Editor store hooks for API docs
  const operations = useEditorStore((state) => state.operations);
  const openApiDocDrawer = useEditorStore((state) => state.openApiDocDrawer);
  const closeApiDocDrawer = useEditorStore((state) => state.closeApiDocDrawer);
  const setSelectedOperation = useEditorStore((state) => state.setSelectedOperation);
  const loadOperations = useEditorStore((state) => state.loadOperations);
  const loadOpenApiSpec = useEditorStore((state) => state.loadOpenApiSpec);

  // Read state from URL query parameters
  const selectedWorkflow = searchParams.get('workflow') || '';
  const layout = (searchParams.get('layout') as LayoutOption['value']) || 'dagre';
  const urlOperationId = searchParams.get('operation');

  const selectedWorkflowLabel = useMemo(
    () => workflows.find((wf) => wf.workflow_id === selectedWorkflow)?.workflow_id ?? '',
    [selectedWorkflow, workflows],
  );

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
        // If no workflow is selected in URL, select the first one
        const currentWorkflow = searchParams.get('workflow');
        if (!currentWorkflow || !newWorkflows.find((w) => w.workflow_id === currentWorkflow)) {
          setSearchParams((params) => {
            params.set('workflow', newWorkflows[0].workflow_id);
            return params;
          });
        }
      } else {
        setGraph(null);
        setStatus({ message: `No workflows found for project ${project}`, type: 'error' });
        return;
      }

      setStatus({ message: `Loaded ${newWorkflows.length} workflow(s)`, type: 'success' });
    } catch (error) {
      setStatus({ message: `Error: ${(error as Error).message}`, type: 'error' });
    }
  }, [project, searchParams, setSearchParams]);

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

  useEffect(() => {
    if (project) {
      void fetchWorkflows();
      void loadOperations(project);
      void loadOpenApiSpec(project);
    }
  }, [project, fetchWorkflows, loadOperations, loadOpenApiSpec]);

  useEffect(() => {
    if (selectedWorkflow && project) {
      void fetchGraph(selectedWorkflow);
    }
  }, [fetchGraph, selectedWorkflow, project]);

  // Sync operation and drawer state from URL to store
  useEffect(() => {
    if (urlOperationId && operations.length > 0) {
      const operation = operations.find((op) => op.operation_id === urlOperationId);
      if (operation) {
        setSelectedOperation(operation);
        openApiDocDrawer(urlOperationId);
      }
    } else {
      closeApiDocDrawer();
    }
  }, [urlOperationId, operations, setSelectedOperation, openApiDocDrawer, closeApiDocDrawer]);

  const handleWorkflowChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setSearchParams((params) => {
      params.set('workflow', event.target.value);
      return params;
    });
    setSelectedNode(null);
  };

  const handleLayoutChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setSearchParams((params) => {
      params.set('layout', event.target.value);
      return params;
    });
  };

  const handleRefresh = () => {
    setSelectedNode(null);
    void fetchWorkflows();
  };

  const handleOpenApiDocs = useCallback(
    (operationId: string) => {
      setSearchParams((params) => {
        params.set('operation', operationId);
        return params;
      });
    },
    [setSearchParams],
  );

  const handleCloseApiDocs = useCallback(() => {
    setSearchParams((params) => {
      params.delete('operation');
      return params;
    });
  }, [setSearchParams]);

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
        <DetailsPanel nodeData={selectedNode} onViewDocs={handleOpenApiDocs} />
      </div>

      <StatusBar message={status.message} type={status.type} />

      {/* API Documentation Drawer */}
      <ApiDocDrawer onClose={handleCloseApiDocs} />

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
