import React, { useState, useMemo } from 'react';
import { useEditorStore } from '../stores/editorStore';
import type { OperationInfo } from '../types/editor';

export const OperationList: React.FC = () => {
  const { operations, selectedOperation, setSelectedOperation, addOperationToWorkflow } =
    useEditorStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [methodFilter, setMethodFilter] = useState<string[]>([]);

  // Filter operations based on search and method
  const filteredOperations = useMemo(() => {
    return operations.filter((op) => {
      const matchesSearch =
        searchQuery === '' ||
        op.operation_id.toLowerCase().includes(searchQuery.toLowerCase()) ||
        op.path.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (op.summary && op.summary.toLowerCase().includes(searchQuery.toLowerCase()));

      const matchesMethod =
        methodFilter.length === 0 || methodFilter.includes(op.method.toUpperCase());

      return matchesSearch && matchesMethod;
    });
  }, [operations, searchQuery, methodFilter]);

  // Get unique methods
  const availableMethods = useMemo(() => {
    const methods = new Set(operations.map((op) => op.method.toUpperCase()));
    return Array.from(methods).sort();
  }, [operations]);

  const toggleMethodFilter = (method: string) => {
    setMethodFilter((prev) =>
      prev.includes(method) ? prev.filter((m) => m !== method) : [...prev, method],
    );
  };

  const handleOperationClick = (operation: OperationInfo) => {
    setSelectedOperation(operation);
  };

  const handleAddToWorkflow = (operation: OperationInfo, e: React.MouseEvent) => {
    e.stopPropagation();
    addOperationToWorkflow(operation);
  };

  const getMethodColor = (method: string): string => {
    switch (method.toUpperCase()) {
      case 'GET':
        return '#61affe';
      case 'POST':
        return '#49cc90';
      case 'PUT':
        return '#fca130';
      case 'DELETE':
        return '#f93e3e';
      case 'PATCH':
        return '#50e3c2';
      default:
        return '#999';
    }
  };

  return (
    <div className="operation-list">
      <div className="operation-list-header">
        <h3>Operations</h3>
        <input
          type="text"
          placeholder="Search operations..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="search-input"
        />
      </div>

      <div className="method-filters">
        {availableMethods.map((method) => (
          <button
            key={method}
            className={`method-filter ${methodFilter.includes(method) ? 'active' : ''}`}
            style={{
              borderColor: getMethodColor(method),
              backgroundColor: methodFilter.includes(method) ? getMethodColor(method) : 'transparent',
              color: methodFilter.includes(method) ? 'white' : getMethodColor(method),
            }}
            onClick={() => toggleMethodFilter(method)}
          >
            {method}
          </button>
        ))}
      </div>

      <div className="operation-items">
        {filteredOperations.length === 0 ? (
          <div className="no-operations">
            {operations.length === 0 ? 'No operations available' : 'No operations match filters'}
          </div>
        ) : (
          filteredOperations.map((operation) => (
            <div
              key={operation.operation_id}
              className={`operation-item ${selectedOperation?.operation_id === operation.operation_id ? 'selected' : ''}`}
              onClick={() => handleOperationClick(operation)}
            >
              <div className="operation-header">
                <span
                  className="operation-method"
                  style={{ backgroundColor: getMethodColor(operation.method) }}
                >
                  {operation.method}
                </span>
                <span className="operation-path">{operation.path}</span>
              </div>
              <div className="operation-info">
                <div className="operation-id">{operation.operation_id}</div>
                {operation.summary && <div className="operation-summary">{operation.summary}</div>}
              </div>
              <button
                className="add-button"
                onClick={(e) => handleAddToWorkflow(operation, e)}
                title="Add to workflow"
              >
                + Add
              </button>
            </div>
          ))
        )}
      </div>

      <style>{`
        .operation-list {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: #f8f9fa;
          border-right: 1px solid #dee2e6;
        }

        .operation-list-header {
          padding: 1rem;
          border-bottom: 1px solid #dee2e6;
        }

        .operation-list-header h3 {
          margin: 0 0 0.75rem 0;
          font-size: 1.1rem;
          color: #212529;
        }

        .search-input {
          width: 100%;
          padding: 0.5rem;
          border: 1px solid #ced4da;
          border-radius: 4px;
          font-size: 0.875rem;
        }

        .search-input:focus {
          outline: none;
          border-color: #0d6efd;
          box-shadow: 0 0 0 0.2rem rgba(13, 110, 253, 0.25);
        }

        .method-filters {
          display: flex;
          gap: 0.5rem;
          padding: 0.75rem 1rem;
          border-bottom: 1px solid #dee2e6;
          flex-wrap: wrap;
        }

        .method-filter {
          padding: 0.25rem 0.75rem;
          border: 1px solid;
          border-radius: 4px;
          font-size: 0.75rem;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.2s;
        }

        .method-filter:hover {
          opacity: 0.8;
        }

        .operation-items {
          flex: 1;
          overflow-y: auto;
          padding: 0.5rem;
        }

        .no-operations {
          padding: 2rem 1rem;
          text-align: center;
          color: #6c757d;
          font-size: 0.875rem;
        }

        .operation-item {
          background: white;
          border: 1px solid #dee2e6;
          border-radius: 6px;
          padding: 0.75rem;
          margin-bottom: 0.5rem;
          cursor: pointer;
          transition: all 0.2s;
          position: relative;
        }

        .operation-item:hover {
          border-color: #0d6efd;
          box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }

        .operation-item.selected {
          border-color: #0d6efd;
          background: #e7f1ff;
        }

        .operation-header {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-bottom: 0.5rem;
        }

        .operation-method {
          display: inline-block;
          padding: 0.2rem 0.5rem;
          border-radius: 3px;
          color: white;
          font-size: 0.7rem;
          font-weight: 700;
          text-transform: uppercase;
          min-width: 50px;
          text-align: center;
        }

        .operation-path {
          font-family: 'Courier New', monospace;
          font-size: 0.85rem;
          color: #495057;
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .operation-info {
          margin-left: 0;
        }

        .operation-id {
          font-size: 0.8rem;
          color: #6c757d;
          font-weight: 500;
        }

        .operation-summary {
          font-size: 0.75rem;
          color: #868e96;
          margin-top: 0.25rem;
        }

        .add-button {
          position: absolute;
          top: 0.5rem;
          right: 0.5rem;
          padding: 0.25rem 0.75rem;
          background: #28a745;
          color: white;
          border: none;
          border-radius: 4px;
          font-size: 0.75rem;
          cursor: pointer;
          opacity: 0;
          transition: opacity 0.2s;
        }

        .operation-item:hover .add-button {
          opacity: 1;
        }

        .add-button:hover {
          background: #218838;
        }
      `}</style>
    </div>
  );
};
