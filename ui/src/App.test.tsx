import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type { GraphData, WorkflowSummary } from './types/graph';

vi.mock('./lib/cytoscape', () => {
  const layout = { run: vi.fn() };
  const elements = { remove: vi.fn() };
  const cy = {
    on: vi.fn(),
    layout: vi.fn(() => layout),
    elements: vi.fn(() => elements),
    add: vi.fn(),
    destroy: vi.fn(),
    fit: vi.fn(),
  };

  return {
    createCytoscape: vi.fn(() => cy),
  };
});

describe('App', () => {
  const workflowsPayload: { workflows: WorkflowSummary[] } = {
    workflows: [
      { workflow_id: 'wf-1', steps: 2 },
      { workflow_id: 'wf-2', steps: 3 },
    ],
  };

  const graphPayload: GraphData = {
    nodes: [
      { id: 'n1', label: 'Node 1', method: 'GET' },
      { id: 'n2', label: 'Node 2', method: 'POST' },
    ],
    edges: [{ source: 'n1', target: 'n2', edge_type: 'Sequential', label: 'next' }],
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    const stringifyRequest = (request: RequestInfo | URL) => {
      if (typeof request === 'string') return request;
      if (request instanceof URL) return request.toString();
      if (request instanceof Request) return request.url;
      return '[unknown request]';
    };

    const fetchMock = vi.fn<typeof fetch>((url) => {
      if (url === '/api/workflows') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(workflowsPayload),
          status: 200,
          statusText: 'OK',
        }) as Promise<Response>;
      }

      if (typeof url === 'string' && url.startsWith('/api/graph/')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(graphPayload),
          status: 200,
          statusText: 'OK',
        }) as Promise<Response>;
      }

      return Promise.reject(new Error(`Unexpected url ${stringifyRequest(url)}`));
    });

    globalThis.fetch = fetchMock;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads workflows and selects the first workflow automatically', async () => {
    render(<App />);

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledWith('/api/workflows'));
    expect(await screen.findByText('wf-1 (2 steps)')).toBeInTheDocument();
    expect(screen.getByTestId('workflow-select')).toHaveValue('wf-1');

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledWith('/api/graph/wf-1'));
    expect(screen.getByRole('status')).toHaveTextContent('Rendered 2 nodes and 1 edges');
  });

  it('allows changing layout selection', async () => {
    render(<App />);
    await waitFor(() => expect(screen.getAllByTestId('layout-select')[0]).toBeInTheDocument());

    const [layoutSelect] = screen.getAllByTestId('layout-select');
    fireEvent.change(layoutSelect, { target: { value: 'grid' } });
    expect(layoutSelect).toHaveValue('grid');
  });
});
