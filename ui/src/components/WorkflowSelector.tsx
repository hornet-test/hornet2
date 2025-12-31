import React from 'react';
import type { WorkflowInfo } from '../types/editor';

interface WorkflowSelectorProps {
  workflows: WorkflowInfo[];
  currentWorkflowId: string | null;
  isDirty: boolean;
  isSaving: boolean;
  onWorkflowSelect: (workflowId: string) => void;
  onNewWorkflow: () => void;
  onSaveWorkflow: () => void;
}

export const WorkflowSelector: React.FC<WorkflowSelectorProps> = ({
  workflows,
  currentWorkflowId,
  isDirty,
  isSaving,
  onWorkflowSelect,
  onNewWorkflow,
  onSaveWorkflow,
}) => {
  const handleSelectChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const selectedId = event.target.value;
    if (selectedId && selectedId !== currentWorkflowId) {
      onWorkflowSelect(selectedId);
    }
  };

  return (
    <div className="workflow-selector">
      <select
        className="workflow-dropdown"
        value={currentWorkflowId || ''}
        onChange={handleSelectChange}
        disabled={isSaving}
      >
        <option value="" disabled>
          {workflows.length === 0 ? 'No workflows available' : 'Select workflow...'}
        </option>
        {workflows.map((workflow) => (
          <option key={workflow.workflow_id} value={workflow.workflow_id}>
            {workflow.title || workflow.workflow_id} ({workflow.steps} steps)
          </option>
        ))}
      </select>

      <button
        className="workflow-action-btn new-btn"
        onClick={onNewWorkflow}
        disabled={isSaving}
        title="Create new workflow"
      >
        New
      </button>

      <button
        className="workflow-action-btn save-btn"
        onClick={onSaveWorkflow}
        disabled={!isDirty || isSaving}
        title={isDirty ? 'Save workflow' : 'No changes to save'}
      >
        {isSaving ? (
          <>
            <span className="spinner-small"></span>
            Saving...
          </>
        ) : (
          <>Save{isDirty ? ' *' : ''}</>
        )}
      </button>

      <style>{getStyles()}</style>
    </div>
  );
};

function getStyles() {
  return `
    .workflow-selector {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-left: 1.5rem;
    }

    .workflow-dropdown {
      padding: 0.5rem 0.75rem;
      border: 1px solid #dee2e6;
      border-radius: 6px;
      background: white;
      color: #495057;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      min-width: 200px;
      max-width: 300px;
    }

    .workflow-dropdown:hover:not(:disabled) {
      border-color: #0d6efd;
    }

    .workflow-dropdown:focus {
      outline: none;
      border-color: #0d6efd;
      box-shadow: 0 0 0 3px rgba(13, 110, 253, 0.1);
    }

    .workflow-dropdown:disabled {
      background: #e9ecef;
      cursor: not-allowed;
      opacity: 0.6;
    }

    .workflow-action-btn {
      padding: 0.5rem 1rem;
      border: 1px solid #dee2e6;
      border-radius: 6px;
      font-size: 0.875rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .workflow-action-btn:disabled {
      cursor: not-allowed;
      opacity: 0.5;
    }

    .new-btn {
      background: #f8f9fa;
      color: #495057;
    }

    .new-btn:hover:not(:disabled) {
      background: #e9ecef;
      border-color: #6c757d;
    }

    .save-btn {
      background: #0d6efd;
      color: white;
      border-color: #0d6efd;
    }

    .save-btn:hover:not(:disabled) {
      background: #0b5ed7;
      border-color: #0a58ca;
    }

    .save-btn:disabled {
      background: #6c757d;
      border-color: #6c757d;
    }

    .spinner-small {
      width: 14px;
      height: 14px;
      border: 2px solid rgba(255, 255, 255, 0.3);
      border-top: 2px solid white;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
      0% {
        transform: rotate(0deg);
      }
      100% {
        transform: rotate(360deg);
      }
    }
  `;
}
