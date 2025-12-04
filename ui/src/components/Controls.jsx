import React from 'react';

export function Controls({ workflows, selectedWorkflow, onWorkflowChange, layout, onLayoutChange, onRefresh, layoutOptions }) {
  return (
    <div className="controls">
      <div className="control-group">
        <label htmlFor="workflow-select">Select Workflow:</label>
        <select id="workflow-select" value={selectedWorkflow} onChange={onWorkflowChange} data-testid="workflow-select">
          {workflows.length === 0 && <option value="">Loading...</option>}
          {workflows.map((wf) => (
            <option key={wf.workflow_id} value={wf.workflow_id}>
              {`${wf.workflow_id} (${wf.steps} steps)`}
            </option>
          ))}
        </select>
      </div>
      <div className="control-group">
        <label htmlFor="layout-select">Layout:</label>
        <select id="layout-select" value={layout} onChange={onLayoutChange} data-testid="layout-select">
          {layoutOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
      <button className="btn" onClick={onRefresh} type="button">
        Refresh
      </button>
    </div>
  );
}
