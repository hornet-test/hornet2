import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.jsx';

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
  const workflowsPayload = {
    workflows: [
      { workflow_id: 'wf-1', steps: 2 },
      { workflow_id: 'wf-2', steps: 3 },
    ],
  };

  const graphPayload = {
    nodes: [
      { id: 'n1', label: 'Node 1', method: 'GET' },
      { id: 'n2', label: 'Node 2', method: 'POST' },
    ],
    edges: [
      { source: 'n1', target: 'n2', edge_type: 'Sequential', label: 'next' },
    ],
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    global.fetch = vi.fn((url) => {
      if (url === '/api/workflows') {
        return Promise.resolve({
          ok: true,
          json: async () => workflowsPayload,
          status: 200,
          statusText: 'OK',
        });
      }

      if (url.startsWith('/api/graph/')) {
        return Promise.resolve({
          ok: true,
          json: async () => graphPayload,
          status: 200,
          statusText: 'OK',
        });
      }

      return Promise.reject(new Error(`Unexpected url ${url}`));
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads workflows and selects the first workflow automatically', async () => {
    render(<App />);

    await waitFor(() => expect(global.fetch).toHaveBeenCalledWith('/api/workflows'));
    expect(await screen.findByText('wf-1 (2 steps)')).toBeInTheDocument();
    expect(screen.getByTestId('workflow-select')).toHaveValue('wf-1');

    await waitFor(() => expect(global.fetch).toHaveBeenCalledWith('/api/graph/wf-1'));
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
