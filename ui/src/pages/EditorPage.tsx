import React, { useEffect, useState } from 'react';
import { OperationList } from '../components/OperationList';
import { WorkflowView } from '../components/WorkflowView';
import { YamlEditor } from '../components/YamlEditor';
import { useEditorStore } from '../stores/editorStore';

export const EditorPage: React.FC = () => {
  const { loadOperations, isLoading, error } = useEditorStore();
  const [viewMode, setViewMode] = useState<'visual' | 'yaml' | 'split'>('split');

  useEffect(() => {
    void loadOperations();
  }, [loadOperations]);

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

  return (
    <div className="editor-page">
      <div className="editor-toolbar">
        <h2>Arazzo Workflow Editor</h2>
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
          <button className={viewMode === 'yaml' ? 'active' : ''} onClick={() => setViewMode('yaml')}>
            YAML
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
        </div>
      </div>

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

    .editor-toolbar h2 {
      margin: 0;
      font-size: 1.5rem;
      color: #212529;
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
  `;
}
