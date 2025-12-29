import React, { useEffect, useState, useCallback } from 'react';
import { OperationList } from '../components/OperationList';
import { WorkflowView } from '../components/WorkflowView';
import { YamlEditor } from '../components/YamlEditor';
import { DataSourcePanel } from '../components/DataSourcePanel';
import { SuggestionPanel } from '../components/SuggestionPanel';
import { WorkflowSelector } from '../components/WorkflowSelector';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { useEditorStore } from '../stores/editorStore';
import { useProjectStore } from '../stores/projectStore';

export const EditorPage: React.FC = () => {
  const { currentProject } = useProjectStore();
  const {
    isLoading,
    error,
    operations,
    selectedStepId,
    dataSourcePanelOpen,
    toggleDataSourcePanel,
    getAvailableDataSources,
    addDataMapping,
    removeDataMapping,
    arazzoSpec,
    suggestionPanelOpen,
    toggleSuggestionPanel,
    dataFlowSuggestions,
    applySuggestion,
    dismissSuggestion,
    loadOperations,
    workflows,
    currentWorkflowId,
    isDirty,
    isSaving,
    saveError,
    loadWorkflows,
    loadWorkflow,
    saveWorkflow,
    createNewWorkflow,
  } = useEditorStore();
  const [viewMode, setViewMode] = useState<'visual' | 'yaml' | 'split'>('split');
  const [showUnsavedWarning, setShowUnsavedWarning] = useState(false);
  const [pendingAction, setPendingAction] = useState<'load' | 'new' | null>(null);
  const [pendingWorkflowId, setPendingWorkflowId] = useState<string | null>(null);

  useEffect(() => {
    if (currentProject) {
      void loadOperations(currentProject);
      void loadWorkflows(currentProject);
    }
  }, [loadOperations, loadWorkflows, currentProject]);

  // Handle workflow selection
  const handleWorkflowSelect = useCallback(
    (workflowId: string) => {
      if (isDirty) {
        setPendingAction('load');
        setPendingWorkflowId(workflowId);
        setShowUnsavedWarning(true);
      } else {
        void loadWorkflow(currentProject!, workflowId);
      }
    },
    [isDirty, currentProject, loadWorkflow],
  );

  // Handle new workflow
  const handleNewWorkflow = useCallback(() => {
    if (isDirty) {
      setPendingAction('new');
      setShowUnsavedWarning(true);
    } else {
      createNewWorkflow();
    }
  }, [isDirty, createNewWorkflow]);

  // Handle save workflow
  const handleSaveWorkflow = useCallback(() => {
    if (currentProject) {
      void saveWorkflow(currentProject);
    }
  }, [currentProject, saveWorkflow]);

  // Handle confirm unsaved action
  const handleConfirmUnsavedAction = useCallback(() => {
    if (pendingAction === 'load' && pendingWorkflowId && currentProject) {
      void loadWorkflow(currentProject, pendingWorkflowId);
    } else if (pendingAction === 'new') {
      createNewWorkflow();
    }
    setShowUnsavedWarning(false);
    setPendingAction(null);
    setPendingWorkflowId(null);
  }, [pendingAction, pendingWorkflowId, currentProject, loadWorkflow, createNewWorkflow]);

  if (isLoading) {
    return (
      <div className="editor-page">
        <div className="loading-state">
          <div className="spinner"></div>
          <p>Loading operations...</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  if (error) {
    return (
      <div className="editor-page">
        <div className="error-state">
          <h3>Error</h3>
          <p>{error}</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  // Get available data sources
  const dataSources = getAvailableDataSources();

  // Get target operation for selected step
  const getTargetOperation = () => {
    if (!selectedStepId || !arazzoSpec || arazzoSpec.workflows.length === 0) {
      return null;
    }
    const workflow = arazzoSpec.workflows[0];
    const step = workflow.steps.find((s) => s.stepId === selectedStepId);
    if (!step || !step.operationId) {
      return null;
    }
    return operations.find((op) => op.operation_id === step.operationId) || null;
  };

  const targetOperation = getTargetOperation();

  return (
    <div className="editor-page">
      <div className="editor-toolbar">
        <div className="toolbar-left">
          <h2>Arazzo Workflow Editor</h2>
          {currentProject && (
            <WorkflowSelector
              workflows={workflows}
              currentWorkflowId={currentWorkflowId}
              isDirty={isDirty}
              isSaving={isSaving}
              onWorkflowSelect={handleWorkflowSelect}
              onNewWorkflow={handleNewWorkflow}
              onSaveWorkflow={handleSaveWorkflow}
            />
          )}
        </div>
        <div className="toolbar-controls">
          <div className="view-mode-toggle">
            <button
              className={viewMode === 'visual' ? 'active' : ''}
              onClick={() => setViewMode('visual')}
            >
              Visual
            </button>
            <button
              className={viewMode === 'split' ? 'active' : ''}
              onClick={() => setViewMode('split')}
            >
              Split
            </button>
            <button
              className={viewMode === 'yaml' ? 'active' : ''}
              onClick={() => setViewMode('yaml')}
            >
              YAML
            </button>
          </div>
          <button
            className={`data-source-toggle ${dataSourcePanelOpen ? 'active' : ''}`}
            onClick={toggleDataSourcePanel}
          >
            🔗 Data Sources
            {selectedStepId && dataSources.length > 0 && (
              <span className="badge">{dataSources.length}</span>
            )}
          </button>
          <button
            className={`data-source-toggle ${suggestionPanelOpen ? 'active' : ''}`}
            onClick={toggleSuggestionPanel}
          >
            💡 Suggestions
            {dataFlowSuggestions.length > 0 && (
              <span className="badge">{dataFlowSuggestions.length}</span>
            )}
          </button>
        </div>
      </div>

      <div className="editor-layout">
        <div className="sidebar">
          <OperationList />
        </div>

        <div className="main-area">
          {viewMode === 'visual' && (
            <div className="full-panel">
              <WorkflowView />
            </div>
          )}

          {viewMode === 'yaml' && (
            <div className="full-panel">
              <YamlEditor />
            </div>
          )}

          {viewMode === 'split' && (
            <>
              <div className="split-panel">
                <WorkflowView />
              </div>
              <div className="split-panel">
                <YamlEditor />
              </div>
            </>
          )}

          {dataSourcePanelOpen && (
            <div className="data-source-sidebar">
              <DataSourcePanel
                dataSources={dataSources}
                selectedStepId={selectedStepId}
                targetOperation={targetOperation}
                onMapData={addDataMapping}
                onRemoveMapping={removeDataMapping}
              />
            </div>
          )}

          {suggestionPanelOpen && (
            <div className="data-source-sidebar">
              <SuggestionPanel
                suggestions={dataFlowSuggestions}
                onApplySuggestion={applySuggestion}
                onDismissSuggestion={dismissSuggestion}
              />
            </div>
          )}
        </div>
      </div>

      {saveError && (
        <div className="save-error-banner">
          <span>⚠️ {saveError}</span>
          <button className="error-close-btn" onClick={() => useEditorStore.setState({ saveError: null })}>
            ✕
          </button>
        </div>
      )}

      {showUnsavedWarning && (
        <ConfirmDialog
          isOpen={showUnsavedWarning}
          title="未保存の変更"
          message="保存されていない変更があります。破棄しますか？"
          confirmLabel="破棄"
          cancelLabel="キャンセル"
          variant="warning"
          onConfirm={handleConfirmUnsavedAction}
          onCancel={() => setShowUnsavedWarning(false)}
        />
      )}

      <style>{getStyles()}</style>
    </div>
  );
};

function getStyles() {
  return `
    .editor-page {
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }

    .loading-state,
    .error-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100vh;
      color: #6c757d;
    }

    .spinner {
      width: 40px;
      height: 40px;
      border: 4px solid #f3f3f3;
      border-top: 4px solid #0d6efd;
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      0% {
        transform: rotate(0deg);
      }
      100% {
        transform: rotate(360deg);
      }
    }

    .error-state h3 {
      color: #dc3545;
      margin: 0 0 0.5rem 0;
    }

    .error-state p {
      margin: 0;
      font-family: 'Courier New', monospace;
      background: #f8d7da;
      padding: 1rem;
      border-radius: 4px;
      border: 1px solid #f5c6cb;
    }

    .editor-toolbar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 1.5rem;
      background: white;
      border-bottom: 2px solid #dee2e6;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
    }

    .toolbar-left {
      display: flex;
      align-items: center;
      gap: 0;
    }

    .editor-toolbar h2 {
      margin: 0;
      font-size: 1.5rem;
      color: #212529;
    }

    .save-error-banner {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.75rem 1.5rem;
      background: #f8d7da;
      color: #842029;
      border-bottom: 1px solid #f5c2c7;
      font-size: 0.875rem;
    }

    .error-close-btn {
      background: none;
      border: none;
      color: #842029;
      font-size: 1.25rem;
      cursor: pointer;
      padding: 0;
      width: 24px;
      height: 24px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 4px;
    }

    .error-close-btn:hover {
      background: rgba(132, 32, 41, 0.1);
    }

    .toolbar-controls {
      display: flex;
      gap: 1rem;
      align-items: center;
    }

    .view-mode-toggle {
      display: flex;
      gap: 0;
      background: #f8f9fa;
      border-radius: 6px;
      overflow: hidden;
      border: 1px solid #dee2e6;
    }

    .view-mode-toggle button {
      padding: 0.5rem 1.25rem;
      background: transparent;
      border: none;
      color: #495057;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      border-right: 1px solid #dee2e6;
    }

    .view-mode-toggle button:last-child {
      border-right: none;
    }

    .view-mode-toggle button:hover {
      background: #e9ecef;
    }

    .view-mode-toggle button.active {
      background: #0d6efd;
      color: white;
    }

    .editor-layout {
      display: flex;
      flex: 1;
      overflow: hidden;
    }

    .sidebar {
      width: 350px;
      flex-shrink: 0;
      overflow: hidden;
    }

    .main-area {
      flex: 1;
      display: flex;
      overflow: hidden;
    }

    .full-panel {
      flex: 1;
      overflow: hidden;
    }

    .split-panel {
      flex: 1;
      overflow: hidden;
      border-right: 1px solid #dee2e6;
    }

    .split-panel:last-child {
      border-right: none;
    }

    .data-source-sidebar {
      width: 350px;
      flex-shrink: 0;
      overflow: hidden;
    }

    .data-source-toggle {
      padding: 0.5rem 1rem;
      background: #f8f9fa;
      color: #495057;
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

    .data-source-toggle:hover {
      background: #e9ecef;
      border-color: #0d6efd;
      color: #0d6efd;
    }

    .data-source-toggle.active {
      background: #0d6efd;
      color: white;
      border-color: #0d6efd;
    }

    .data-source-toggle .badge {
      background: #28a745;
      color: white;
      padding: 0.15rem 0.5rem;
      border-radius: 10px;
      font-size: 0.7rem;
      font-weight: 700;
    }

    .data-source-toggle.active .badge {
      background: white;
      color: #0d6efd;
    }
  `;
}
