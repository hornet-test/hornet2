import React, { useMemo } from 'react';
import { useEditorStore } from '../stores/editorStore';
import type { ArazzoStep } from '../types/editor';

export const WorkflowView: React.FC = () => {
  const { arazzoSpec, selectedStepId, setSelectedStepId, operations } = useEditorStore();

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
              <p>Sequence of operations for this scenario.</p>
            </div>

            <div className="steps-list">
              {steps.map((step, index) => {
                const operation = getOperationDetails(step.operationId);
                const isSelected = selectedStepId === step.stepId;
                const stepClass = isSelected ? 'step-content selected' : 'step-content';

                return (
                  <div key={step.stepId} className="step-card">
                    <div className="step-number">{index + 1}</div>

                    <div className={stepClass} onClick={() => handleStepClick(step.stepId)}>
                      <div className="step-header">
                        <span className="step-id">{step.stepId}</span>
                        <span className="operation-ref">
                          {operation
                            ? `${operation.method} ${operation.path}`
                            : step.operationId || 'No Op'}
                        </span>
                      </div>

                      <div className="step-summary">
                        {operation?.summary || step.description || 'No description'}
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
                                  {/* Simplified rendering of criteria object */}
                                  {criteria.context} {criteria.condition} {String(criteria.value)}
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
