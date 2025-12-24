import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach, afterEach, type Mock } from 'vitest';
import * as matchers from '@testing-library/jest-dom/matchers';
expect.extend(matchers);
import { OperationList } from '../OperationList';
import { useEditorStore } from '../../stores/editorStore';

// Mock the store
vi.mock('../../stores/editorStore');

const mockOperations = [
  {
    operation_id: 'getUsers',
    method: 'GET',
    path: '/users',
    summary: 'Get all users',
    description: 'Returns a list of users',
    parameters: [],
    response_codes: ['200'],
  },
  {
    operation_id: 'createUser',
    method: 'POST',
    path: '/users',
    summary: 'Create a user',
    description: 'Creates a new user',
    parameters: [],
    response_codes: ['201'],
  },
];

describe('OperationList', () => {
  const mockSetSelectedOperation = vi.fn();
  const mockAddOperationToWorkflow = vi.fn();

  beforeEach(() => {
    (useEditorStore as unknown as Mock).mockReturnValue({
      operations: mockOperations,
      selectedOperation: null,
      setSelectedOperation: mockSetSelectedOperation,
      addOperationToWorkflow: mockAddOperationToWorkflow,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  test('renders operation list', () => {
    render(<OperationList />);
    expect(screen.getByText('Operations')).toBeInTheDocument();
    expect(screen.getByText('getUsers')).toBeInTheDocument();
    expect(screen.getByText('createUser')).toBeInTheDocument();
  });

  test('filters operations by search query', () => {
    render(<OperationList />);
    const searchInput = screen.getByPlaceholderText(/search/i);

    fireEvent.change(searchInput, { target: { value: 'create' } });

    expect(screen.queryByText('getUsers')).not.toBeInTheDocument();
    expect(screen.getByText('createUser')).toBeInTheDocument();
  });

  test('filters operations by method', () => {
    render(<OperationList />);
    const postFilter = screen.getByRole('button', { name: 'POST' });

    fireEvent.click(postFilter);

    expect(screen.queryByText('getUsers')).not.toBeInTheDocument();
    expect(screen.getByText('createUser')).toBeInTheDocument();
  });

  test('selects operation on click', () => {
    render(<OperationList />);
    const operationItems = screen.getAllByText('getUsers');
    // Find the one that is likely the list item, e.g. checking parent class or just pick first if reliable
    // The previous error showed multiple because text is split?
    // The DOM showed: <div class="operation-id">getUsers</div> inside <div class="operation-info">
    // Maybe `screen.getAllByText('getUsers')` returns the div.
    // Why multiple? maybe multiple operations? No, mock has 1 getUsers.
    // The DOM dump showed duplicates of the entire list.
    // This suggests cleanup didn't happen.
    // Using getAllByText[0] is a workaround.
    const operationItem = operationItems[0].closest('.operation-item');

    fireEvent.click(operationItem!);

    expect(mockSetSelectedOperation).toHaveBeenCalledWith(mockOperations[0]);
  });

  test('adds operation to workflow on add button click', () => {
    render(<OperationList />);
    // Hover is hard to simulate for CSS, but we can target the button directly
    // Ideally we might need to simulate mouse over but the button is continuously in DOM, just hidden?
    // Based on CSS: display is not none, opacity is 0. So it is in DOM.
    const addButton = screen.getAllByTitle('Add to workflow')[0];

    fireEvent.click(addButton);

    expect(mockAddOperationToWorkflow).toHaveBeenCalledWith(mockOperations[0]);
    // Should stop propagation and not select (though selecting might happen if logic allows, usually add doesn't toggle selection)
  });
});
