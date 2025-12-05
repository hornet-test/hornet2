import { create } from 'zustand';
import yaml from 'js-yaml';
import type { ArazzoSpec, OperationInfo, ValidationError } from '../types/editor';

interface EditorState {
  // Data
  operations: OperationInfo[];
  arazzoSpec: ArazzoSpec | null;
  yamlContent: string;
  validationErrors: ValidationError[];
  isValid: boolean;

  // UI state
  selectedOperation: OperationInfo | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadOperations: () => Promise<void>;
  setArazzoSpec: (spec: ArazzoSpec) => void;
  setYamlContent: (yaml: string) => void;
  syncYamlToSpec: () => void;
  syncSpecToYaml: () => void;
  validateYaml: (yaml: string) => Promise<void>;
  setSelectedOperation: (operation: OperationInfo | null) => void;
  addOperationToWorkflow: (operation: OperationInfo) => void;
  reset: () => void;
}

const initialArazzoSpec: ArazzoSpec = {
  arazzo: '1.0.0',
  info: {
    title: 'New Workflow',
    description: 'Created with Hornet2 Editor',
    version: '1.0.0',
  },
  sourceDescriptions: [],
  workflows: [],
};

export const useEditorStore = create<EditorState>((set, get) => ({
  // Initial state
  operations: [],
  arazzoSpec: initialArazzoSpec,
  yamlContent: yaml.dump(initialArazzoSpec),
  validationErrors: [],
  isValid: true,
  selectedOperation: null,
  isLoading: false,
  error: null,

  // Load operations from API
  loadOperations: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch('/api/editor/operations');
      if (!response.ok) {
        throw new Error(`Failed to load operations: ${response.statusText}`);
      }
      const data = (await response.json()) as { operations: OperationInfo[] };
      set({ operations: data.operations, isLoading: false });
    } catch (error) {
      set({
        error: (error as Error).message,
        isLoading: false,
      });
    }
  },

  // Set Arazzo spec and sync to YAML
  setArazzoSpec: (spec: ArazzoSpec) => {
    set({ arazzoSpec: spec });
    get().syncSpecToYaml();
  },

  // Set YAML content (called when user edits YAML)
  setYamlContent: (yamlStr: string) => {
    set({ yamlContent: yamlStr });
  },

  // Sync YAML to Arazzo spec
  syncYamlToSpec: () => {
    const { yamlContent } = get();
    try {
      const parsed = yaml.load(yamlContent) as ArazzoSpec;
      set({
        arazzoSpec: parsed,
        validationErrors: [],
        isValid: true,
        error: null,
      });
      // Validate with backend
      void get().validateYaml(yamlContent);
    } catch (error) {
      set({
        error: `YAML parse error: ${(error as Error).message}`,
        isValid: false,
      });
    }
  },

  // Sync Arazzo spec to YAML
  syncSpecToYaml: () => {
    const { arazzoSpec } = get();
    if (arazzoSpec) {
      try {
        const yamlStr = yaml.dump(arazzoSpec, {
          indent: 2,
          lineWidth: 100,
          noRefs: true,
        });
        set({ yamlContent: yamlStr });
      } catch (error) {
        set({ error: `Failed to serialize YAML: ${(error as Error).message}` });
      }
    }
  },

  // Validate YAML with backend
  validateYaml: async (yamlStr: string) => {
    try {
      const response = await fetch('/api/editor/validate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ yaml: yamlStr }),
      });

      if (!response.ok) {
        throw new Error(`Validation failed: ${response.statusText}`);
      }

      const result = (await response.json()) as {
        valid: boolean;
        errors: ValidationError[];
      };

      set({
        validationErrors: result.errors,
        isValid: result.valid,
      });
    } catch (error) {
      set({ error: `Validation error: ${(error as Error).message}` });
    }
  },

  // Set selected operation
  setSelectedOperation: (operation: OperationInfo | null) => {
    set({ selectedOperation: operation });
  },

  // Add operation to workflow
  addOperationToWorkflow: (operation: OperationInfo) => {
    const { arazzoSpec } = get();
    if (!arazzoSpec) return;

    // Create a new workflow if none exists
    let workflows = [...arazzoSpec.workflows];
    if (workflows.length === 0) {
      workflows = [
        {
          workflowId: 'workflow-1',
          summary: 'New Workflow',
          description: '',
          steps: [],
        },
      ];
    }

    // Add step to the first workflow
    const workflow = workflows[0];
    const stepId = `step-${workflow.steps.length + 1}`;

    // Find the first 2xx response code, or fall back to the first available code, or 200
    const getDefaultStatusCode = (codes: string[]): number => {
      if (codes.length === 0) return 200;

      // Find first 2xx code
      const successCode = codes.find((code) => code.startsWith('2'));
      if (successCode) return parseInt(successCode, 10);

      // Fall back to first code
      return parseInt(codes[0], 10);
    };

    const defaultStatusCode = getDefaultStatusCode(operation.response_codes);

    workflow.steps.push({
      stepId,
      description: operation.summary || `${operation.method} ${operation.path}`,
      operationId: operation.operation_id,
      parameters: [],
      successCriteria: [
        {
          context: '$statusCode',
          condition: '==',
          value: defaultStatusCode,
        },
      ],
    });

    const newSpec = {
      ...arazzoSpec,
      workflows,
    };

    get().setArazzoSpec(newSpec);
  },

  // Reset editor
  reset: () => {
    set({
      arazzoSpec: initialArazzoSpec,
      yamlContent: yaml.dump(initialArazzoSpec),
      validationErrors: [],
      isValid: true,
      selectedOperation: null,
      error: null,
    });
  },
}));
