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
        return '#0d6efd';
      case 'POST':
        return '#28a745';
      case 'PUT':
        return '#fd7e14';
      case 'DELETE':
        return '#dc3545';
      case 'PATCH':
        return '#20c997';
      default:
        return '#6c757d';
    }
  };

  return (
    <div className="operation-list">
      <div className="operation-list-header">
        <h3>Operations</h3>
        <input
          type="text"
          placeholder="Search..."
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
              backgroundColor: methodFilter.includes(method) ? getMethodColor(method) : 'white',
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
            {operations.length === 0 ? (
              <p>No operations available</p>
            ) : (
              <p>No operations match filters</p>
            )}
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
    </div>
  );
};
