import React from 'react';
import { useEditorStore } from '../stores/editorStore';
import type { ArazzoWorkflow, ArazzoStep } from '../types/editor';

export const WorkflowView: React.FC = () => {
  const { arazzoSpec } = useEditorStore();

  if (!arazzoSpec || arazzoSpec.workflows.length === 0) {
    return (
      <div className="workflow-view">
        <div className="empty-state">
          <h3>No Workflows Yet</h3>
          <p>Add operations from the list to create a workflow</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  return (
    <div className="workflow-view">
      <div className="workflow-header">
        <h3>Workflow Visualization</h3>
        <div className="workflow-count">{arazzoSpec.workflows.length} workflow(s)</div>
      </div>

      <div className="workflows-container">
        {arazzoSpec.workflows.map((workflow, index) => (
          <WorkflowCard key={workflow.workflowId} workflow={workflow} index={index} />
        ))}
      </div>

      <style>{getStyles()}</style>
    </div>
  );
};

interface WorkflowCardProps {
  workflow: ArazzoWorkflow;
  index: number;
}

const WorkflowCard: React.FC<WorkflowCardProps> = ({ workflow, index }) => {
  return (
    <div className="workflow-card">
      <div className="workflow-card-header">
        <h4>{workflow.summary || workflow.workflowId}</h4>
        <span className="step-count">{workflow.steps.length} steps</span>
      </div>
      {workflow.description && <div className="workflow-description">{workflow.description}</div>}

      <div className="steps-list">
        {workflow.steps.map((step, stepIndex) => (
          <StepCard key={step.stepId} step={step} stepIndex={stepIndex} />
        ))}
      </div>
    </div>
  );
};

interface StepCardProps {
  step: ArazzoStep;
  stepIndex: number;
}

const StepCard: React.FC<StepCardProps> = ({ step, stepIndex }) => {
  return (
    <div className="step-card">
      <div className="step-number">{stepIndex + 1}</div>
      <div className="step-content">
        <div className="step-header">
          <span className="step-id">{step.stepId}</span>
          {step.operationId && <span className="operation-id">{step.operationId}</span>}
        </div>
        {step.description && <div className="step-description">{step.description}</div>}

        {step.parameters && step.parameters.length > 0 && (
          <div className="step-details">
            <div className="details-label">Parameters:</div>
            {step.parameters.map((param, idx) => (
              <div key={idx} className="param-item">
                <span className="param-name">
                  {param.name} ({param.in})
                </span>
                <span className="param-value">{JSON.stringify(param.value)}</span>
              </div>
            ))}
          </div>
        )}

        {step.successCriteria && step.successCriteria.length > 0 && (
          <div className="step-details">
            <div className="details-label">Success Criteria:</div>
            {step.successCriteria.map((criteria, idx) => (
              <div key={idx} className="criteria-item">
                {criteria.context} {criteria.condition} {JSON.stringify(criteria.value)}
              </div>
            ))}
          </div>
        )}
      </div>
      {stepIndex < 999 && <div className="step-arrow">↓</div>}
    </div>
  );
};

function getStyles() {
  return `
    .workflow-view {
      display: flex;
      flex-direction: column;
      height: 100%;
      background: #f8f9fa;
      overflow: hidden;
    }

    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: #6c757d;
    }

    .empty-state h3 {
      margin: 0 0 0.5rem 0;
      font-size: 1.25rem;
    }

    .empty-state p {
      margin: 0;
      font-size: 0.9rem;
    }

    .workflow-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid #dee2e6;
      background: white;
    }

    .workflow-header h3 {
      margin: 0;
      font-size: 1rem;
      color: #212529;
    }

    .workflow-count {
      font-size: 0.875rem;
      color: #6c757d;
      font-weight: 500;
    }

    .workflows-container {
      flex: 1;
      overflow-y: auto;
      padding: 1rem;
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
      margin-bottom: 0.5rem;
    }

    .workflow-card-header h4 {
      margin: 0;
      font-size: 1.1rem;
      color: #212529;
    }

    .step-count {
      background: #e7f1ff;
      color: #0d6efd;
      padding: 0.25rem 0.75rem;
      border-radius: 12px;
      font-size: 0.8rem;
      font-weight: 600;
    }

    .workflow-description {
      margin-bottom: 1rem;
      color: #6c757d;
      font-size: 0.9rem;
    }

    .steps-list {
      margin-top: 1rem;
    }

    .step-card {
      display: flex;
      gap: 1rem;
      margin-bottom: 0.5rem;
      position: relative;
    }

    .step-number {
      flex-shrink: 0;
      width: 32px;
      height: 32px;
      background: #0d6efd;
      color: white;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 600;
      font-size: 0.85rem;
    }

    .step-content {
      flex: 1;
      background: #f8f9fa;
      border: 1px solid #dee2e6;
      border-radius: 6px;
      padding: 0.75rem;
    }

    .step-header {
      display: flex;
      gap: 0.75rem;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .step-id {
      font-weight: 600;
      color: #212529;
      font-size: 0.9rem;
    }

    .operation-id {
      font-family: 'Courier New', monospace;
      font-size: 0.8rem;
      color: #0d6efd;
      background: #e7f1ff;
      padding: 0.2rem 0.5rem;
      border-radius: 3px;
    }

    .step-description {
      color: #6c757d;
      font-size: 0.85rem;
      margin-bottom: 0.5rem;
    }

    .step-details {
      margin-top: 0.75rem;
      font-size: 0.8rem;
    }

    .details-label {
      font-weight: 600;
      color: #495057;
      margin-bottom: 0.25rem;
    }

    .param-item,
    .criteria-item {
      padding: 0.25rem 0.5rem;
      background: white;
      border-radius: 3px;
      margin-bottom: 0.25rem;
      font-family: 'Courier New', monospace;
      font-size: 0.75rem;
    }

    .param-name {
      color: #0d6efd;
      font-weight: 600;
      margin-right: 0.5rem;
    }

    .param-value {
      color: #495057;
    }

    .criteria-item {
      color: #495057;
    }

    .step-arrow {
      position: absolute;
      left: 15px;
      top: 45px;
      color: #ced4da;
      font-size: 1.5rem;
    }
  `;
}
