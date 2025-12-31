import React, { useMemo } from 'react';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useEditorStore } from '../stores/editorStore';
import type { ArazzoStep } from '../types/editor';

interface SortableStepProps {
  step: ArazzoStep;
  index: number;
  isSelected: boolean;
  operationDetails?: {
    method: string;
    path: string;
    summary?: string;
  };
  workflowId: string;
  onStepClick: (stepId: string) => void;
  onRemoveStep: (workflowId: string, stepId: string) => void;
}

const SortableStep: React.FC<SortableStepProps> = ({
  step,
  index,
  isSelected,
  operationDetails,
  workflowId,
  onStepClick,
  onRemoveStep,
}) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: step.stepId,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const stepClass = isSelected ? 'step-content selected' : 'step-content';

  const handleRemove = (e: React.MouseEvent) => {
    e.stopPropagation();
    onRemoveStep(workflowId, step.stepId);
  };

  return (
    <div ref={setNodeRef} style={style} className="step-card">
      <div className="step-number">{index + 1}</div>

      <div className="drag-handle" {...attributes} {...listeners}>
        ⋮⋮
      </div>

      <div className={stepClass} onClick={() => onStepClick(step.stepId)}>
        <div className="step-header">
          <span className="step-id">{step.stepId}</span>
          <span className="operation-ref">
            {operationDetails
              ? `${operationDetails.method} ${operationDetails.path}`
              : step.operationId || 'No Op'}
          </span>
        </div>

        <div className="step-summary">
          {operationDetails?.summary || step.description || 'No description'}
        </div>

        {isSelected && (
          <div className="step-details">
            {step.parameters && step.parameters.length > 0 && (
              <div className="step-section">
                <div className="details-label">Parameters</div>
                {step.parameters.map((param, i) => (
                  <div key={i} className="param-item">
                    <span className="param-name">{param.name}:</span>
                    <span className="param-value">{String(param.value)}</span>
                  </div>
                ))}
              </div>
            )}

            {step.successCriteria && step.successCriteria.length > 0 && (
              <div className="step-section">
                <div className="details-label">Success Criteria</div>
                {step.successCriteria.map((criteria, i) => (
                  <div key={i} className="criteria-item">
                    {criteria.context} {criteria.condition} {String(criteria.value)}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      <button className="delete-button" onClick={handleRemove} title="Remove step">
        ✕
      </button>

      <style>{`
        .step-card {
          display: flex;
          align-items: flex-start;
          gap: 0.5rem;
          margin-bottom: 0.75rem;
          position: relative;
        }

        .step-number {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 28px;
          height: 28px;
          background: #0d6efd;
          color: white;
          border-radius: 50%;
          font-weight: 600;
          font-size: 0.75rem;
          flex-shrink: 0;
          margin-top: 0.5rem;
        }

        .drag-handle {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 24px;
          padding: 0.5rem 0;
          cursor: grab;
          color: #6c757d;
          font-size: 1rem;
          line-height: 1;
          flex-shrink: 0;
          user-select: none;
        }

        .drag-handle:active {
          cursor: grabbing;
        }

        .drag-handle:hover {
          color: #0d6efd;
        }

        .step-content {
          flex: 1;
          padding: 0.75rem 1rem;
          background: white;
          border: 2px solid #dee2e6;
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .step-content:hover {
          border-color: #0d6efd;
          box-shadow: 0 2px 4px rgba(13, 110, 253, 0.1);
        }

        .step-content.selected {
          border-color: #0d6efd;
          background: #f8f9ff;
          box-shadow: 0 0 0 3px rgba(13, 110, 253, 0.1);
        }

        .step-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.5rem;
        }

        .step-id {
          font-weight: 600;
          color: #212529;
          font-size: 0.875rem;
        }

        .operation-ref {
          font-family: 'Courier New', monospace;
          font-size: 0.75rem;
          color: #6c757d;
          background: #f8f9fa;
          padding: 0.15rem 0.5rem;
          border-radius: 3px;
        }

        .step-summary {
          color: #495057;
          font-size: 0.875rem;
          line-height: 1.4;
        }

        .step-details {
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid #dee2e6;
        }

        .step-section {
          margin-bottom: 0.75rem;
        }

        .step-section:last-child {
          margin-bottom: 0;
        }

        .details-label {
          font-weight: 600;
          font-size: 0.75rem;
          color: #6c757d;
          text-transform: uppercase;
          margin-bottom: 0.5rem;
        }

        .param-item {
          display: flex;
          gap: 0.5rem;
          margin-bottom: 0.25rem;
          font-size: 0.875rem;
        }

        .param-name {
          font-weight: 600;
          color: #495057;
        }

        .param-value {
          font-family: 'Courier New', monospace;
          color: #6c757d;
          background: #f8f9fa;
          padding: 0.1rem 0.4rem;
          border-radius: 3px;
        }

        .criteria-item {
          font-family: 'Courier New', monospace;
          font-size: 0.875rem;
          color: #495057;
          background: #f8f9fa;
          padding: 0.25rem 0.5rem;
          border-radius: 3px;
          margin-bottom: 0.25rem;
        }

        .delete-button {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 24px;
          height: 24px;
          background: none;
          border: none;
          color: #dc3545;
          font-size: 1.25rem;
          cursor: pointer;
          border-radius: 4px;
          flex-shrink: 0;
          margin-top: 0.5rem;
          transition: all 0.2s;
        }

        .delete-button:hover {
          background: #dc3545;
          color: white;
        }
      `}</style>
    </div>
  );
};

export const WorkflowView: React.FC = () => {
  const { arazzoSpec, selectedStepId, setSelectedStepId, operations, removeStep, reorderSteps } =
    useEditorStore();

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const handleStepClick = (stepId: string) => {
    setSelectedStepId(stepId);
  };

  const getGroupedWorkflows = useMemo(() => {
    if (!arazzoSpec) return {};
    const grouped: Record<string, ArazzoStep[]> = {};
    arazzoSpec.workflows.forEach((w) => {
      grouped[w.workflowId] = w.steps;
    });
    return grouped;
  }, [arazzoSpec]);

  const getOperationDetails = (operationId?: string) => {
    if (!operationId) return undefined;
    return operations.find((op) => op.operation_id === operationId);
  };

  const handleDragEnd = (event: DragEndEvent, workflowId: string, steps: ArazzoStep[]) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = steps.findIndex((step) => step.stepId === active.id);
      const newIndex = steps.findIndex((step) => step.stepId === over.id);

      reorderSteps(workflowId, oldIndex, newIndex);
    }
  };

  if (!arazzoSpec || arazzoSpec.workflows.length === 0) {
    return (
      <div className="workflow-view">
        <div className="workflow-header">
          <h3>Current Workflow</h3>
          <span className="workflow-count">0 Flows</span>
        </div>
        <div className="workflows-container">
          <div className="empty-state">
            <h3>No Steps Added</h3>
            <p>
              Select an operation from the list and click &quot;Add&quot; to start building your
              workflow.
            </p>
          </div>
        </div>

        <style>{getStyles()}</style>
      </div>
    );
  }

  return (
    <div className="workflow-view">
      <div className="workflow-header">
        <h3>Current Workflow</h3>
        <span className="workflow-count">{Object.keys(getGroupedWorkflows).length} Flows</span>
      </div>

      <div className="workflows-container">
        {Object.entries(getGroupedWorkflows).map(([wfId, steps]) => (
          <div key={wfId} className="workflow-card">
            <div className="workflow-card-header">
              <h4>{wfId}</h4>
              <span className="step-count">{steps.length} Steps</span>
            </div>

            <div className="workflow-description">
              <p>Drag steps to reorder them in the workflow.</p>
            </div>

            <div className="steps-list">
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={(event) => handleDragEnd(event, wfId, steps)}
              >
                <SortableContext
                  items={steps.map((s) => s.stepId)}
                  strategy={verticalListSortingStrategy}
                >
                  {steps.map((step, index) => {
                    const operation = getOperationDetails(step.operationId);
                    const isSelected = selectedStepId === step.stepId;

                    return (
                      <SortableStep
                        key={step.stepId}
                        step={step}
                        index={index}
                        isSelected={isSelected}
                        operationDetails={operation}
                        workflowId={wfId}
                        onStepClick={handleStepClick}
                        onRemoveStep={removeStep}
                      />
                    );
                  })}
                </SortableContext>
              </DndContext>
            </div>
          </div>
        ))}
      </div>

      <style>{getStyles()}</style>
    </div>
  );
};

function getStyles() {
  return `
    .workflow-view {
      display: flex;
      flex-direction: column;
      height: 100%;
      background: white;
    }

    .workflow-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid #dee2e6;
      background: #f8f9fa;
    }

    .workflow-header h3 {
      margin: 0;
      font-size: 1rem;
      color: #212529;
    }

    .workflow-count {
      font-size: 0.875rem;
      color: #6c757d;
      font-weight: 600;
    }

    .workflows-container {
      flex: 1;
      overflow-y: auto;
      padding: 1rem;
    }

    .empty-state {
      text-align: center;
      padding: 3rem 2rem;
      color: #6c757d;
    }

    .empty-state h3 {
      margin: 0 0 0.5rem 0;
      font-size: 1.25rem;
      color: #495057;
    }

    .empty-state p {
      margin: 0;
      font-size: 0.875rem;
    }

    .workflow-card {
      background: white;
      border: 1px solid #dee2e6;
      border-radius: 8px;
      padding: 1rem;
      margin-bottom: 1rem;
    }

    .workflow-card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.75rem;
      padding-bottom: 0.75rem;
      border-bottom: 2px solid #dee2e6;
    }

    .workflow-card-header h4 {
      margin: 0;
      font-size: 1.125rem;
      color: #212529;
    }

    .step-count {
      font-size: 0.875rem;
      color: #0d6efd;
      font-weight: 600;
      background: #e7f1ff;
      padding: 0.25rem 0.75rem;
      border-radius: 12px;
    }

    .workflow-description {
      margin-bottom: 1rem;
    }

    .workflow-description p {
      margin: 0;
      font-size: 0.875rem;
      color: #6c757d;
      font-style: italic;
    }

    .steps-list {
      display: flex;
      flex-direction: column;
    }
  `;
}
