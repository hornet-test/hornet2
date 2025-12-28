import type { SelectedNode } from '../types/graph';

type DetailsPanelProps = {
  nodeData: SelectedNode | null;
};

export function DetailsPanel({ nodeData }: DetailsPanelProps) {
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
          </>
        )}
      </div>
    </div>
  );
}
