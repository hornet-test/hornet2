import React, { useEffect, useState, useRef } from 'react';
import { useEditorStore } from '../stores/editorStore';
import ApiDocViewer from './ApiDocViewer';

interface ApiDocDrawerProps {
  onClose?: () => void;
}

export const ApiDocDrawer: React.FC<ApiDocDrawerProps> = ({ onClose }) => {
  const {
    apiDocDrawerOpen,
    apiDocDrawerWidth,
    apiDocDrawerOperationId,
    setApiDocDrawerWidth,
    openApiSpec,
    isLoadingOpenApi,
  } = useEditorStore();

  const closeApiDocDrawer = useEditorStore((state) => state.closeApiDocDrawer);

  const [isDragging, setIsDragging] = useState(false);
  const drawerRef = useRef<HTMLDivElement>(null);

  // Handle resize drag
  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setApiDocDrawerWidth(newWidth);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, setApiDocDrawerWidth]);

  // Handle close action
  const handleClose = () => {
    if (onClose) {
      onClose();
    } else {
      closeApiDocDrawer();
    }
  };

  // Handle Escape key
  useEffect(() => {
    if (!apiDocDrawerOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [apiDocDrawerOpen]); // eslint-disable-line react-hooks/exhaustive-deps

  // Focus management
  useEffect(() => {
    if (apiDocDrawerOpen && drawerRef.current) {
      drawerRef.current.focus();
    }
  }, [apiDocDrawerOpen]);

  if (!apiDocDrawerOpen) {
    return null;
  }

  return (
    <>
      {/* Overlay */}
      <div className="api-doc-drawer-overlay" onClick={handleClose} />

      {/* Drawer */}
      <div
        ref={drawerRef}
        className="api-doc-drawer"
        style={{ width: `${apiDocDrawerWidth}px` }}
        role="dialog"
        aria-label="API Documentation"
        tabIndex={-1}
      >
        {/* Resize Handle */}
        <div
          className={`resize-handle ${isDragging ? 'dragging' : ''}`}
          onMouseDown={() => setIsDragging(true)}
        />

        {/* Header */}
        <div className="api-doc-drawer-header">
          <h3>API Documentation</h3>
          <button
            className="close-button"
            onClick={handleClose}
            aria-label="Close documentation"
          >
            ✕
          </button>
        </div>

        {/* Content */}
        <div className="api-doc-drawer-content">
          {isLoadingOpenApi ? (
            <div className="loading-state">
              <div className="spinner" />
              <p>Loading OpenAPI specification...</p>
            </div>
          ) : openApiSpec && apiDocDrawerOperationId ? (
            <ApiDocViewer
              openApiSpec={openApiSpec}
              operationId={apiDocDrawerOperationId}
            />
          ) : (
            <div className="error-state">
              <p>{!openApiSpec ? 'OpenAPI specification not available' : 'Operation not found'}</p>
            </div>
          )}
        </div>
      </div>

      <style>{getStyles()}</style>
    </>
  );
};

function getStyles() {
  return `
    .api-doc-drawer-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.2);
      z-index: 7999;
      animation: fadeIn 300ms ease-out;
    }

    @keyframes fadeIn {
      from {
        opacity: 0;
      }
      to {
        opacity: 1;
      }
    }

    .api-doc-drawer {
      position: fixed;
      right: 0;
      top: 0;
      bottom: 0;
      background: white;
      z-index: 8000;
      box-shadow: -4px 0 12px rgba(0, 0, 0, 0.15);
      display: flex;
      flex-direction: column;
      animation: slideIn 300ms ease-out;
    }

    @keyframes slideIn {
      from {
        transform: translateX(100%);
      }
      to {
        transform: translateX(0);
      }
    }

    .api-doc-drawer:focus {
      outline: none;
    }

    .resize-handle {
      position: absolute;
      left: 0;
      top: 0;
      bottom: 0;
      width: 10px;
      cursor: ew-resize;
      background: transparent;
      transition: background 200ms;
      z-index: 1;
    }

    .resize-handle:hover,
    .resize-handle.dragging {
      background: rgba(13, 110, 253, 0.3);
    }

    .api-doc-drawer-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 1.5rem;
      border-bottom: 1px solid #dee2e6;
      background: white;
      height: 60px;
      flex-shrink: 0;
    }

    .api-doc-drawer-header h3 {
      margin: 0;
      font-size: 1.1rem;
      font-weight: 600;
      color: #212529;
    }

    .close-button {
      background: none;
      border: none;
      font-size: 1.5rem;
      color: #6c757d;
      cursor: pointer;
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 4px;
      transition: all 0.2s;
    }

    .close-button:hover {
      background: #f8f9fa;
      color: #212529;
    }

    .api-doc-drawer-content {
      flex: 1;
      overflow-y: auto;
      background: white;
    }

    .loading-state,
    .error-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 3rem 1.5rem;
      text-align: center;
      color: #6c757d;
    }

    .loading-state .spinner {
      width: 40px;
      height: 40px;
      border: 4px solid rgba(13, 110, 253, 0.1);
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

    .loading-state p,
    .error-state p {
      margin: 0;
      font-size: 0.875rem;
    }
  `;
}

export default ApiDocDrawer;
