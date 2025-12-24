import React from 'react';
import type { DataFlowSuggestion } from '../utils/dataFlowAnalyzer';

interface SuggestionPanelProps {
  suggestions: DataFlowSuggestion[];
  onApplySuggestion: (suggestion: DataFlowSuggestion) => void;
  onDismissSuggestion: (suggestion: DataFlowSuggestion) => void;
}

export const SuggestionPanel: React.FC<SuggestionPanelProps> = ({
  suggestions,
  onApplySuggestion,
  onDismissSuggestion,
}) => {
  if (suggestions.length === 0) {
    return (
      <div className="suggestion-panel">
        <div className="panel-header">
          <h3>💡 Suggestions</h3>
        </div>
        <div className="empty-state">
          <p>No data flow suggestions available.</p>
          <p className="hint">Add more steps to see suggested parameter mappings.</p>
        </div>
        <style>{getStyles()}</style>
      </div>
    );
  }

  const getConfidenceColor = (confidence: string): string => {
    switch (confidence) {
      case 'high':
        return '#28a745';
      case 'medium':
        return '#ffc107';
      case 'low':
        return '#6c757d';
      default:
        return '#6c757d';
    }
  };

  const getConfidenceLabel = (confidence: string): string => {
    switch (confidence) {
      case 'high':
        return 'High Confidence';
      case 'medium':
        return 'Medium Confidence';
      case 'low':
        return 'Low Confidence';
      default:
        return 'Unknown';
    }
  };

  return (
    <div className="suggestion-panel">
      <div className="panel-header">
        <h3>💡 Suggestions</h3>
        <span className="suggestion-count">{suggestions.length} found</span>
      </div>

      <div className="suggestions-list">
        {suggestions.map((suggestion, index) => (
          <div key={index} className="suggestion-item">
            <div className="suggestion-header">
              <span
                className="confidence-badge"
                style={{ backgroundColor: getConfidenceColor(suggestion.confidence) }}
              >
                {getConfidenceLabel(suggestion.confidence)}
              </span>
            </div>

            <div className="suggestion-flow">
              <div className="flow-item">
                <div className="flow-label">From</div>
                <div className="flow-value">
                  <span className="step-id">{suggestion.sourceStep}</span>
                  <span className="output-name">{suggestion.sourceOutput}</span>
                </div>
              </div>

              <div className="flow-arrow">→</div>

              <div className="flow-item">
                <div className="flow-label">To</div>
                <div className="flow-value">
                  <span className="step-id">{suggestion.targetStep}</span>
                  <span className="param-name">
                    {suggestion.targetParameter}{' '}
                    <span className="param-location">({suggestion.targetLocation})</span>
                  </span>
                </div>
              </div>
            </div>

            <div className="suggestion-reason">{suggestion.reason}</div>

            <div className="suggestion-actions">
              <button className="action-button apply" onClick={() => onApplySuggestion(suggestion)}>
                Apply
              </button>
              <button
                className="action-button dismiss"
                onClick={() => onDismissSuggestion(suggestion)}
              >
                Dismiss
              </button>
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
    .suggestion-panel {
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
    }

    .panel-header h3 {
      margin: 0;
      font-size: 1rem;
      color: #212529;
    }

    .suggestion-count {
      font-size: 0.875rem;
      color: #6c757d;
      background: #e9ecef;
      padding: 0.25rem 0.75rem;
      border-radius: 12px;
      font-weight: 600;
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

    .suggestions-list {
      flex: 1;
      overflow-y: auto;
      padding: 1rem;
    }

    .suggestion-item {
      background: white;
      border: 1px solid #dee2e6;
      border-radius: 8px;
      padding: 1rem;
      margin-bottom: 1rem;
      transition: all 0.2s;
    }

    .suggestion-item:hover {
      border-color: #0d6efd;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    }

    .suggestion-header {
      margin-bottom: 0.75rem;
    }

    .confidence-badge {
      display: inline-block;
      color: white;
      padding: 0.25rem 0.75rem;
      border-radius: 4px;
      font-size: 0.75rem;
      font-weight: 700;
      text-transform: uppercase;
    }

    .suggestion-flow {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 0.75rem;
      padding: 0.75rem;
      background: #f8f9fa;
      border-radius: 6px;
    }

    .flow-item {
      flex: 1;
    }

    .flow-label {
      font-size: 0.7rem;
      color: #6c757d;
      font-weight: 600;
      text-transform: uppercase;
      margin-bottom: 0.25rem;
    }

    .flow-value {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .step-id {
      font-size: 0.85rem;
      color: #495057;
      font-weight: 600;
    }

    .output-name,
    .param-name {
      font-size: 0.8rem;
      color: #0d6efd;
      font-family: 'Courier New', monospace;
      font-weight: 600;
    }

    .param-location {
      color: #6c757d;
      font-weight: 400;
      font-size: 0.75rem;
    }

    .flow-arrow {
      font-size: 1.5rem;
      color: #0d6efd;
      font-weight: 700;
    }

    .suggestion-reason {
      font-size: 0.85rem;
      color: #495057;
      padding: 0.5rem;
      background: #e7f1ff;
      border-left: 3px solid #0d6efd;
      border-radius: 4px;
      margin-bottom: 0.75rem;
    }

    .suggestion-actions {
      display: flex;
      gap: 0.5rem;
    }

    .action-button {
      flex: 1;
      padding: 0.5rem 1rem;
      border: none;
      border-radius: 4px;
      font-size: 0.875rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
    }

    .action-button.apply {
      background: #28a745;
      color: white;
    }

    .action-button.apply:hover {
      background: #218838;
    }

    .action-button.dismiss {
      background: #e9ecef;
      color: #495057;
    }

    .action-button.dismiss:hover {
      background: #dee2e6;
    }
  `;
}
