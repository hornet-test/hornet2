export type WorkflowSummary = {
  workflow_id: string;
  steps: number;
};

export type GraphNode = {
  id: string;
  label?: string;
  method?: string;
  operation_id?: string;
  path?: string;
  description?: string;
};

export type GraphEdge = {
  source: string;
  target: string;
  edge_type?: string;
  label?: string;
};

export type GraphData = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};

export type StatusMessage = {
  message: string;
  type: 'info' | 'success' | 'error';
};

export type SelectedNode = GraphNode & {
  connectedEdges?: number;
};

export type LayoutOption = {
  value: 'dagre' | 'breadthfirst' | 'circle' | 'grid';
  label: string;
};
