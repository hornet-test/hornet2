import type { SelectedNode } from '../types/graph';

type DetailsPanelProps = {
  nodeData: SelectedNode | null;
  onViewDocs?: (operationId: string) => void;
};

export function DetailsPanel({ nodeData, onViewDocs }: DetailsPanelProps) {
  const connectionCount = nodeData?.connectedEdges ?? 0;

  return (
    <div id="details-panel" className="details-panel">
      <h3>Node Details</h3>
      <div id="details-content">
        {!nodeData && <p className="placeholder">Click on a node to see details</p>}
        {nodeData && (
          <>
            <div className="detail-item">
              <div className="detail-label">Step ID</div>
              <div className="detail-value">
                <code>{nodeData.id}</code>
              </div>
            </div>
            {nodeData.method && (
              <div className="detail-item">
                <div className="detail-label">HTTP Method</div>
                <div className="detail-value">
                  <span className={`http-method ${nodeData.method.toLowerCase()}`}>
                    {nodeData.method}
                  </span>
                </div>
              </div>
            )}
            {nodeData.operation_id && (
              <div className="detail-item">
                <div className="detail-label">Operation ID</div>
                <div className="detail-value">
                  <code>{nodeData.operation_id}</code>
                </div>
              </div>
            )}
            {nodeData.path && (
              <div className="detail-item">
                <div className="detail-label">Path</div>
                <div className="detail-value">
                  <code>{nodeData.path}</code>
                </div>
              </div>
            )}
            {nodeData.description && (
              <div className="detail-item">
                <div className="detail-label">Description</div>
                <div className="detail-value">{nodeData.description}</div>
              </div>
            )}
            {connectionCount > 0 && (
              <div className="detail-item">
                <div className="detail-label">Connections</div>
                <div className="detail-value">{connectionCount} edge(s)</div>
              </div>
            )}

            {nodeData.operation_id && onViewDocs && (
              <button
                className="view-docs-button"
                onClick={() => onViewDocs(nodeData.operation_id!)}
              >
                📖 View API Documentation
              </button>
            )}
          </>
        )}
      </div>

      <style>{getStyles()}</style>
    </div>
  );
}

function getStyles() {
  return `
    .view-docs-button {
      width: 100%;
      margin-top: 1rem;
      padding: 0.75rem;
      background: #0d6efd;
      color: white;
      border: none;
      border-radius: 6px;
      font-size: 0.875rem;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.2s;
    }

    .view-docs-button:hover {
      background: #0b5ed7;
    }
  `;
}
