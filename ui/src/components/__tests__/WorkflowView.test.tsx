import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach, afterEach, type Mock } from 'vitest';
import * as matchers from '@testing-library/jest-dom/matchers';
expect.extend(matchers);
import { WorkflowView } from '../WorkflowView';
import { useEditorStore } from '../../stores/editorStore';

// Mock the store
vi.mock('../../stores/editorStore');

const mockOperations = [
  {
    operation_id: 'getUsers',
    method: 'GET',
    path: '/users',
    summary: 'Get all users',
  },
];

const mockWorkflowSteps = [
  {
    stepId: 'step1',
    operationId: 'getUsers',
    description: 'First step',
    workflowId: 'default',
  },
  {
    stepId: 'step2',
    operationId: 'createUser', // No mock op for this, to test fallback
    workflowId: 'default',
  },
];

describe('WorkflowView', () => {
  const mockSetSelectedStep = vi.fn();

  beforeEach(() => {
    (useEditorStore as unknown as Mock).mockReturnValue({
      arazzoSpec: {
        workflows: [
          {
            workflowId: 'default',
            steps: mockWorkflowSteps,
          },
        ],
      },
      selectedStepId: null,
      setSelectedStepId: mockSetSelectedStep,
      operations: mockOperations,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  test('renders empty state when no steps', () => {
    (useEditorStore as unknown as Mock).mockReturnValue({
      arazzoSpec: { workflows: [] },
      selectedStepId: null,
      setSelectedStepId: mockSetSelectedStep,
      operations: [],
    });

    render(<WorkflowView />);
    expect(screen.getByText(/No Steps Added/i)).toBeInTheDocument();
  });

  test('renders workflow steps', () => {
    render(<WorkflowView />);
    expect(screen.getByText('Current Workflow')).toBeInTheDocument();
    expect(screen.getByText('default')).toBeInTheDocument();
    expect(screen.getByText('2 Steps')).toBeInTheDocument();
  });

  test('displays operation details in step', () => {
    render(<WorkflowView />);
    // operationId 'getUsers' matches mockOperations[0] which has method GET
    expect(screen.getByText('GET /users')).toBeInTheDocument();
    // 'First step' description
    expect(screen.getByText('Get all users')).toBeInTheDocument();
  });

  test('displays fallback for missing operation details', () => {
    render(<WorkflowView />);
    // step2 has operationId 'createUser' but no operation in store
    expect(screen.getByText('createUser')).toBeInTheDocument();
  });

  test('selects step on click', () => {
    render(<WorkflowView />);
    const step1Elements = screen.getAllByText('step1');
    const step1 = step1Elements[0].closest('.step-content');

    fireEvent.click(step1!); // Use step-content class wrapper

    expect(mockSetSelectedStep).toHaveBeenCalledWith('step1');
  });
});
