import React, { useState } from 'react';
import type { DataSource } from '../stores/editorStore';
import type { OperationInfo } from '../types/editor';

interface DataSourcePanelProps {
  dataSources: DataSource[];
  selectedStepId: string | null;
  targetOperation: OperationInfo | null;
  onMapData: (dataSource: DataSource, targetParam: { name: string; location: string }) => void;
  onRemoveMapping: (
    dataSource: DataSource,
    targetParam: { name: string; location: string },
  ) => void;
}

export const DataSourcePanel: React.FC<DataSourcePanelProps> = ({
  dataSources,
  selectedStepId,
  targetOperation,
  onMapData,
  onRemoveMapping,
}) => {
  const [expandedSource, setExpandedSource] = useState<string | null>(null);

  if (!selectedStepId) {
    return (
      <div className="data-source-panel">
        <div className="panel-header">
          <h3>🔗 Available Data Sources</h3>
        </div>
        <div className="empty-state">
          <p>No step selected.</p>
          <p className="hint">Select a step to see available data sources.</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  if (dataSources.length === 0) {
    return (
      <div className="data-source-panel">
        <div className="panel-header">
          <h3>🔗 Available Data Sources</h3>
          <span className="step-badge">Step: {selectedStepId}</span>
        </div>
        <div className="empty-state">
          <p>No data sources available.</p>
          <p className="hint">Add previous steps to see available data sources.</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  const getStatusBadge = (source: DataSource) => {
    if (source.suggestionConfidence === 'high') {
      return <span className="status-badge high">⭐ High Match</span>;
    }
    if (source.isOutput) {
      return <span className="status-badge output">✓ Output</span>;
    }
    if (source.suggestionConfidence === 'medium') {
      return <span className="status-badge medium">Similar</span>;
    }
    if (source.suggestionConfidence === 'low') {
      return <span className="status-badge low">Low Match</span>;
    }
    return null;
  };

  const toggleExpand = (sourceKey: string) => {
    setExpandedSource(expandedSource === sourceKey ? null : sourceKey);
  };

  const getSourceKey = (source: DataSource) => {
    return `${source.sourceStepId}:${source.propertyName}`;
  };

  const handleQuickMap = (source: DataSource) => {
    // If already mapped, do nothing (use X button to remove)
    if (source.isMapped) return;

    // Try to find a matching parameter in the target operation
    if (!targetOperation) return;

    // Look for exact name match first
    const matchingParam = targetOperation.parameters.find(
      (p) => p.name.toLowerCase() === source.propertyName.toLowerCase(),
    );

    if (matchingParam) {
      onMapData(source, {
        name: matchingParam.name,
        location: matchingParam.location,
      });
    } else {
      // Expand to show parameter selection
      toggleExpand(getSourceKey(source));
    }
  };

  const handleRemoveMapping = (source: DataSource, e: React.MouseEvent) => {
    e.stopPropagation();
    if (source.mappedToParameter) {
      onRemoveMapping(source, source.mappedToParameter);
    }
  };

  return (
    <div className="data-source-panel">
      <div className="panel-header">
        <h3>🔗 Available Data Sources</h3>
        <span className="step-badge">Target: {selectedStepId}</span>
      </div>

      <div className="sources-list">
        {dataSources.map((source) => {
          const sourceKey = getSourceKey(source);
          const isExpanded = expandedSource === sourceKey;
          const isMapped = source.isMapped || false;

          return (
            <div key={sourceKey} className={`source-item ${isMapped ? 'mapped' : ''}`}>
              <div className="source-header">
                <div className="source-info">
                  <div className="source-step">
                    {source.sourceStepId} &gt; {source.sourceStepDescription}
                  </div>
                  <div className="source-property">
                    <span className="property-name">{source.propertyName}</span>
                    {getStatusBadge(source)}
                    {isMapped && source.mappedToParameter && (
                      <span className="mapped-info">
                        → {source.mappedToParameter.name} ({source.mappedToParameter.location})
                      </span>
                    )}
                  </div>
                </div>
                {isMapped ? (
                  <button
                    className="remove-button"
                    onClick={(e) => handleRemoveMapping(source, e)}
                    title="Remove mapping"
                  >
                    ✕
                  </button>
                ) : (
                  <button className="map-button" onClick={() => handleQuickMap(source)}>
                    Map →
                  </button>
                )}
              </div>

              {isExpanded && targetOperation && !isMapped && (
                <div className="param-selector">
                  <div className="selector-label">Map to parameter:</div>
                  <div className="param-list">
                    {targetOperation.parameters.map((param) => (
                      <button
                        key={`${param.name}:${param.location}`}
                        className="param-option"
                        onClick={() => {
                          onMapData(source, {
                            name: param.name,
                            location: param.location,
                          });
                          setExpandedSource(null);
                        }}
                      >
                        <span className="param-name">{param.name}</span>
                        <span className="param-location">({param.location})</span>
                        {param.required && <span className="required-badge">*</span>}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <style>{getStyles()}</style>
    </div>
  );
};

function getStyles() {
  return `
    .data-source-panel {
      display: flex;
      flex-direction: column;
      height: 100%;
      background: #f8f9fa;
      border-left: 1px solid #dee2e6;
      overflow: hidden;
    }

    .panel-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem;
      border-bottom: 2px solid #dee2e6;
      background: white;
      gap: 0.5rem;
    }

    .panel-header h3 {
      margin: 0;
      font-size: 1rem;
      color: #212529;
      flex: 1;
    }

    .step-badge {
      font-size: 0.75rem;
      color: #495057;
      background: #e9ecef;
      padding: 0.25rem 0.75rem;
      border-radius: 12px;
      font-weight: 600;
      white-space: nowrap;
    }

    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      padding: 2rem;
      text-align: center;
      color: #6c757d;
    }

    .empty-state p {
      margin: 0.5rem 0;
    }

    .empty-state .hint {
      font-size: 0.875rem;
      color: #868e96;
    }

    .sources-list {
      flex: 1;
      overflow-y: auto;
      padding: 1rem;
    }

    .source-item {
      background: white;
      border: 1px solid #dee2e6;
      border-radius: 8px;
      margin-bottom: 0.75rem;
      overflow: hidden;
      transition: all 0.2s;
    }

    .source-item:hover {
      border-color: #0d6efd;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    }

    .source-item.mapped {
      opacity: 0.6;
      background: #f8f9fa;
    }

    .source-item.mapped .source-header {
      cursor: default;
    }

    .source-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.75rem 1rem;
      gap: 1rem;
    }

    .source-info {
      flex: 1;
      min-width: 0;
    }

    .source-step {
      font-size: 0.75rem;
      color: #6c757d;
      font-weight: 600;
      margin-bottom: 0.25rem;
    }

    .source-property {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-wrap: wrap;
    }

    .property-name {
      font-size: 0.875rem;
      color: #0d6efd;
      font-family: 'Courier New', monospace;
      font-weight: 600;
    }

    .status-badge {
      display: inline-block;
      padding: 0.15rem 0.5rem;
      border-radius: 4px;
      font-size: 0.65rem;
      font-weight: 700;
      text-transform: uppercase;
    }

    .status-badge.high {
      background: #28a745;
      color: white;
    }

    .status-badge.output {
      background: #17a2b8;
      color: white;
    }

    .status-badge.medium {
      background: #ffc107;
      color: #212529;
    }

    .status-badge.low {
      background: #6c757d;
      color: white;
    }

    .map-button {
      padding: 0.4rem 1rem;
      background: #0d6efd;
      color: white;
      border: none;
      border-radius: 4px;
      font-size: 0.8rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }

    .map-button:hover {
      background: #0b5ed7;
    }

    .remove-button {
      padding: 0.4rem 0.75rem;
      background: #dc3545;
      color: white;
      border: none;
      border-radius: 4px;
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }

    .remove-button:hover {
      background: #bb2d3b;
    }

    .mapped-info {
      font-size: 0.75rem;
      color: #6c757d;
      font-family: 'Courier New', monospace;
      margin-left: 0.5rem;
    }

    .param-selector {
      padding: 0.75rem 1rem;
      background: #f8f9fa;
      border-top: 1px solid #dee2e6;
    }

    .selector-label {
      font-size: 0.75rem;
      color: #6c757d;
      font-weight: 600;
      margin-bottom: 0.5rem;
      text-transform: uppercase;
    }

    .param-list {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .param-option {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0.75rem;
      background: white;
      border: 1px solid #dee2e6;
      border-radius: 4px;
      cursor: pointer;
      transition: all 0.2s;
      text-align: left;
    }

    .param-option:hover {
      border-color: #0d6efd;
      background: #e7f1ff;
    }

    .param-option .param-name {
      font-size: 0.85rem;
      color: #212529;
      font-family: 'Courier New', monospace;
      font-weight: 600;
    }

    .param-option .param-location {
      font-size: 0.75rem;
      color: #6c757d;
    }

    .required-badge {
      color: #dc3545;
      font-weight: 700;
    }
  `;
}
