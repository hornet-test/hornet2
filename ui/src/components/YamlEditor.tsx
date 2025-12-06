import React, { useEffect, useRef } from 'react';
import Editor, { OnMount } from '@monaco-editor/react';
import { useEditorStore } from '../stores/editorStore';
import type { editor } from 'monaco-editor';

export const YamlEditor: React.FC = () => {
  const { yamlContent, setYamlContent, syncYamlToSpec, validationErrors, isValid } =
    useEditorStore();
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const timeoutRef = useRef<NodeJS.Timeout>();

  const handleEditorDidMount: OnMount = (editor) => {
    editorRef.current = editor;
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value === undefined) return;

    setYamlContent(value);

    // Debounce validation
    if (timeoutRef.current) {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      clearTimeout(timeoutRef.current);
    }
    timeoutRef.current = setTimeout(() => {
      syncYamlToSpec();
    }, 500);
  };

  // Update editor markers for validation errors
  useEffect(() => {
    if (!editorRef.current) return;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const monaco = (window as any).monaco;
    if (!monaco) return;

    const model = editorRef.current.getModel();
    if (!model) return;

    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const markerSeverity = monaco.MarkerSeverity.Error;

    const markers = validationErrors.map((error) => ({
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      severity: markerSeverity,
      startLineNumber: error.line ?? 1,
      startColumn: error.column ?? 1,
      endLineNumber: error.line ?? 1,
      endColumn: error.column ? error.column + 10 : 100,
      message: error.message,
    }));

    // eslint-disable-next-line @typescript-eslint/no-unsafe-call, @typescript-eslint/no-unsafe-member-access
    monaco.editor.setModelMarkers(model, 'yaml-validation', markers);
  }, [validationErrors]);

  return (
    <div className="yaml-editor">
      <div className="yaml-editor-header">
        <h3>YAML Editor</h3>
        <div className="validation-status">
          {isValid ? (
            <span className="status-valid">✓ Valid</span>
          ) : (
            <span className="status-invalid">
              ✗ {validationErrors.length} error{validationErrors.length !== 1 ? 's' : ''}
            </span>
          )}
        </div>
      </div>

      <div className="editor-container">
        <Editor
          height="100%"
          defaultLanguage="yaml"
          value={yamlContent}
          onChange={handleEditorChange}
          onMount={handleEditorDidMount}
          theme="vs-light"
          options={{
            minimap: { enabled: false },
            fontSize: 13,
            lineNumbers: 'on',
            roundedSelection: false,
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            wordWrap: 'on',
            wrappingIndent: 'indent',
          }}
        />
      </div>

      {validationErrors.length > 0 && (
        <div className="validation-errors">
          <div className="errors-header">Validation Errors:</div>
          {validationErrors.map((error, index) => (
            <div key={index} className="error-item">
              {error.line && <span className="error-location">Line {error.line}: </span>}
              <span className="error-message">{error.message}</span>
            </div>
          ))}
        </div>
      )}

      <style>{`
        .yaml-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: white;
        }

        .yaml-editor-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.75rem 1rem;
          border-bottom: 1px solid #dee2e6;
          background: #f8f9fa;
        }

        .yaml-editor-header h3 {
          margin: 0;
          font-size: 1rem;
          color: #212529;
        }

        .validation-status {
          font-size: 0.875rem;
          font-weight: 600;
        }

        .status-valid {
          color: #28a745;
        }

        .status-invalid {
          color: #dc3545;
        }

        .editor-container {
          flex: 1;
          overflow: hidden;
        }

        .validation-errors {
          max-height: 150px;
          overflow-y: auto;
          border-top: 2px solid #dc3545;
          background: #f8d7da;
          padding: 0.75rem 1rem;
        }

        .errors-header {
          font-weight: 600;
          color: #721c24;
          margin-bottom: 0.5rem;
          font-size: 0.875rem;
        }

        .error-item {
          margin-bottom: 0.5rem;
          font-size: 0.8rem;
          color: #721c24;
          padding: 0.25rem;
          background: white;
          border-radius: 3px;
        }

        .error-location {
          font-weight: 600;
          margin-right: 0.5rem;
        }

        .error-message {
          font-family: 'Courier New', monospace;
        }
      `}</style>
    </div>
  );
};
