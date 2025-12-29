import { create } from 'zustand';
import yaml from 'js-yaml';
import type { ArazzoSpec, OperationInfo, ValidationError, WorkflowInfo } from '../types/editor';
import { analyzeDataFlow, type DataFlowSuggestion } from '../utils/dataFlowAnalyzer';

export interface DataSource {
  sourceStepId: string;
  sourceStepDescription: string;
  propertyName: string;
  propertyPath: string; // e.g., "$response.body.token"
  isOutput: boolean; // Already registered as output
  suggestionConfidence?: 'high' | 'medium' | 'low'; // If matched by suggestion algorithm
  isMapped?: boolean; // Already mapped to a parameter in target step
  mappedToParameter?: { name: string; location: string }; // Which parameter it's mapped to
}

interface EditorState {
  // Data
  operations: OperationInfo[];
  arazzoSpec: ArazzoSpec | null;
  yamlContent: string;
  validationErrors: ValidationError[];
  isValid: boolean;
  dataFlowSuggestions: DataFlowSuggestion[];
  dismissedSuggestions: Set<string>;

  // Workflow management
  workflows: WorkflowInfo[];
  currentWorkflowId: string | null;
  isDirty: boolean;
  isSaving: boolean;
  saveError: string | null;

  // UI state
  selectedOperation: OperationInfo | null;
  selectedStepId: string | null;
  dataSourcePanelOpen: boolean;
  suggestionPanelOpen: boolean;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadOperations: (projectName: string) => Promise<void>;
  loadWorkflows: (projectName: string) => Promise<void>;
  loadWorkflow: (projectName: string, workflowId: string) => Promise<void>;
  saveWorkflow: (projectName: string) => Promise<void>;
  createNewWorkflow: () => void;
  setArazzoSpec: (spec: ArazzoSpec) => void;
  setYamlContent: (yaml: string) => void;
  syncYamlToSpec: () => void;
  syncSpecToYaml: () => void;
  validateYaml: (yaml: string) => Promise<void>;
  setSelectedOperation: (operation: OperationInfo | null) => void;
  setSelectedStepId: (stepId: string | null) => void;
  toggleDataSourcePanel: () => void;
  toggleSuggestionPanel: () => void;
  addOperationToWorkflow: (operation: OperationInfo) => void;
  analyzeDataFlows: () => void;
  applySuggestion: (suggestion: DataFlowSuggestion) => void;
  dismissSuggestion: (suggestion: DataFlowSuggestion) => void;
  getAvailableDataSources: () => DataSource[];
  addDataMapping: (dataSource: DataSource, targetParam: { name: string; location: string }) => void;
  removeDataMapping: (
    dataSource: DataSource,
    targetParam: { name: string; location: string },
  ) => void;
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
  dataFlowSuggestions: [],
  dismissedSuggestions: new Set(),
  workflows: [],
  currentWorkflowId: null,
  isDirty: false,
  isSaving: false,
  saveError: null,
  selectedOperation: null,
  selectedStepId: null,
  dataSourcePanelOpen: false,
  suggestionPanelOpen: false,
  isLoading: false,
  error: null,

  // Load operations from API
  loadOperations: async (projectName: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch(`/api/projects/${projectName}/operations`);
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

  // Load workflows from API
  loadWorkflows: async (projectName: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch(`/api/projects/${encodeURIComponent(projectName)}/workflows`);
      if (!response.ok) {
        throw new Error(`Failed to load workflows: ${response.statusText}`);
      }
      const data = (await response.json()) as { workflows: WorkflowInfo[] };
      set({ workflows: data.workflows, isLoading: false });
    } catch (error) {
      set({
        error: (error as Error).message,
        isLoading: false,
      });
    }
  },

  // Load a specific workflow from API
  loadWorkflow: async (projectName: string, workflowId: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch(
        `/api/projects/${encodeURIComponent(projectName)}/workflows/${encodeURIComponent(workflowId)}`,
      );
      if (!response.ok) {
        throw new Error(`Failed to load workflow: ${response.statusText}`);
      }
      const workflow = (await response.json()) as import('../types/editor').ArazzoWorkflow;

      // Convert single workflow to ArazzoSpec format
      const spec: ArazzoSpec = {
        arazzo: '1.0.0',
        info: {
          title: workflow.summary || workflowId,
          description: workflow.description || '',
          version: '1.0.0',
        },
        sourceDescriptions: [],
        workflows: [workflow],
      };

      set({
        arazzoSpec: spec,
        currentWorkflowId: workflowId,
        isDirty: false,
        isLoading: false,
      });

      // Sync to YAML and analyze data flows
      get().syncSpecToYaml();
      get().analyzeDataFlows();
    } catch (error) {
      set({
        error: (error as Error).message,
        isLoading: false,
      });
    }
  },

  // Save current workflow
  saveWorkflow: async (projectName: string) => {
    const { arazzoSpec, currentWorkflowId } = get();
    if (!arazzoSpec || arazzoSpec.workflows.length === 0) {
      set({ saveError: 'No workflow to save' });
      return;
    }

    const workflow = arazzoSpec.workflows[0];

    // Generate workflow ID if not set
    if (!workflow.workflowId) {
      workflow.workflowId = `workflow-${Date.now()}`;
    }

    set({ isSaving: true, saveError: null });

    try {
      const isNewWorkflow = currentWorkflowId === null;
      const url = isNewWorkflow
        ? `/api/projects/${encodeURIComponent(projectName)}/workflows`
        : `/api/projects/${encodeURIComponent(projectName)}/workflows/${encodeURIComponent(workflow.workflowId)}`;

      const method = isNewWorkflow ? 'POST' : 'PUT';

      // Validate workflow ID matches path for updates
      if (!isNewWorkflow && workflow.workflowId !== currentWorkflowId) {
        throw new Error(
          `Workflow ID mismatch: path has "${currentWorkflowId}", body has "${workflow.workflowId}"`,
        );
      }

      // POST expects { workflow: {...} }, PUT expects {...} directly
      const requestBody = isNewWorkflow ? { workflow } : workflow;

      const response = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
      });

      if (!response.ok) {
        // Try to parse RFC 9457 Problem Details
        let errorMessage = `Save failed: ${response.statusText}`;
        try {
          const responseText = await response.text();
          // Try to parse as JSON
          const problem = JSON.parse(responseText) as {
            type?: string;
            title?: string;
            status?: number;
            detail?: string;
          };
          errorMessage = problem.detail || problem.title || errorMessage;
        } catch {
          // If parsing fails, use the default error message
        }
        throw new Error(errorMessage);
      }

      // Success
      set({
        currentWorkflowId: workflow.workflowId,
        isDirty: false,
        isSaving: false,
        saveError: null,
      });

      // Reload workflow list
      void get().loadWorkflows(projectName);
    } catch (error) {
      set({
        saveError: (error as Error).message,
        isSaving: false,
      });
    }
  },

  // Create new workflow
  createNewWorkflow: () => {
    set({
      arazzoSpec: initialArazzoSpec,
      yamlContent: yaml.dump(initialArazzoSpec),
      currentWorkflowId: null,
      isDirty: false,
      selectedStepId: null,
      selectedOperation: null,
      dataFlowSuggestions: [],
      dismissedSuggestions: new Set(),
    });
  },

  // Set Arazzo spec and sync to YAML
  setArazzoSpec: (spec: ArazzoSpec) => {
    set({ arazzoSpec: spec, isDirty: true });
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
        isDirty: true,
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
      const response = await fetch('/api/validate', {
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

  // Set selected step
  setSelectedStepId: (stepId: string | null) => {
    set({ selectedStepId: stepId });
  },

  // Toggle data source panel
  toggleDataSourcePanel: () => {
    set((state) => ({
      dataSourcePanelOpen: !state.dataSourcePanelOpen,
      suggestionPanelOpen: false, // Close suggestion panel when data source panel opens
    }));
  },

  // Toggle suggestion panel
  toggleSuggestionPanel: () => {
    set((state) => ({
      suggestionPanelOpen: !state.suggestionPanelOpen,
      dataSourcePanelOpen: false, // Close data source panel when suggestion panel opens
    }));
  },

  // Get available data sources for the selected step
  getAvailableDataSources: (): DataSource[] => {
    const { arazzoSpec, selectedStepId, operations, dataFlowSuggestions } = get();
    if (!arazzoSpec || !selectedStepId || arazzoSpec.workflows.length === 0) {
      return [];
    }

    const workflow = arazzoSpec.workflows[0]; // Use first workflow
    const selectedStepIndex = workflow.steps.findIndex((s) => s.stepId === selectedStepId);
    if (selectedStepIndex === -1) {
      return [];
    }

    // Get target step's existing parameters
    const targetStep = workflow.steps[selectedStepIndex];
    const targetParameters = targetStep.parameters || [];

    // Build operation map for quick lookup
    const operationMap = new Map<string, OperationInfo>();
    operations.forEach((op) => operationMap.set(op.operation_id, op));

    // Build suggestion map for quick lookup
    const suggestionMap = new Map<string, 'high' | 'medium' | 'low'>();
    dataFlowSuggestions
      .filter((s) => s.targetStep === selectedStepId)
      .forEach((s) => {
        const key = `${s.sourceStep}:${s.sourceOutput}`;
        suggestionMap.set(key, s.confidence);
      });

    const dataSources: DataSource[] = [];

    // Iterate through all previous steps
    for (let i = 0; i < selectedStepIndex; i++) {
      const step = workflow.steps[i];
      const operation = step.operationId ? operationMap.get(step.operationId) : null;
      if (!operation || !operation.response_schema) continue;

      const stepDescription = step.description || `${step.stepId}`;

      // Add all response properties
      for (const propertyName of operation.response_schema.properties) {
        const propertyPath = `$response.body.${propertyName}`;
        const isOutput = step.outputs && Object.keys(step.outputs).includes(propertyName);
        const suggestionKey = `${step.stepId}:${propertyName}`;
        const suggestionConfidence = suggestionMap.get(suggestionKey);

        // Check if this data source is mapped to any parameter
        const expectedValue = `$steps.${step.stepId}.outputs.${propertyName}`;
        let isMapped = false;
        let mappedToParameter: { name: string; location: string } | undefined;

        for (const param of targetParameters) {
          if (param.value === expectedValue) {
            isMapped = true;
            mappedToParameter = { name: param.name, location: param.in };
            break;
          }
        }

        dataSources.push({
          sourceStepId: step.stepId,
          sourceStepDescription: stepDescription,
          propertyName,
          propertyPath,
          isOutput: isOutput || false,
          suggestionConfidence,
          isMapped,
          mappedToParameter,
        });
      }
    }

    // Sort: high confidence > already output > medium confidence > low confidence > others
    dataSources.sort((a, b) => {
      const getScore = (ds: DataSource): number => {
        if (ds.suggestionConfidence === 'high') return 1000;
        if (ds.isOutput) return 900;
        if (ds.suggestionConfidence === 'medium') return 800;
        if (ds.suggestionConfidence === 'low') return 700;
        return 0;
      };
      return getScore(b) - getScore(a);
    });

    return dataSources;
  },

  // Add data mapping (output + parameter)
  addDataMapping: (dataSource: DataSource, targetParam: { name: string; location: string }) => {
    const { arazzoSpec, selectedStepId } = get();
    if (!arazzoSpec || !selectedStepId) return;

    const workflows = [...arazzoSpec.workflows];
    let modified = false;

    for (const workflow of workflows) {
      // Find source step and add output if not exists
      const sourceStep = workflow.steps.find((s) => s.stepId === dataSource.sourceStepId);
      if (sourceStep && !dataSource.isOutput) {
        if (!sourceStep.outputs) {
          sourceStep.outputs = {};
        }
        // Add output if not already present
        if (!sourceStep.outputs[dataSource.propertyName]) {
          sourceStep.outputs[dataSource.propertyName] = dataSource.propertyPath;
          modified = true;
        }
      }

      // Find target step and add parameter
      const targetStep = workflow.steps.find((s) => s.stepId === selectedStepId);
      if (targetStep) {
        if (!targetStep.parameters) {
          targetStep.parameters = [];
        }

        // Check if parameter already exists
        const existingParam = targetStep.parameters.find(
          (p) => p.name === targetParam.name && p.in === targetParam.location,
        );

        if (!existingParam) {
          // Add new parameter with reference to source output
          targetStep.parameters.push({
            name: targetParam.name,
            in: targetParam.location,
            value: `$steps.${dataSource.sourceStepId}.outputs.${dataSource.propertyName}`,
          });
          modified = true;
        }
      }
    }

    if (modified) {
      const newSpec = { ...arazzoSpec, workflows };
      get().setArazzoSpec(newSpec);
      // Re-analyze data flows after adding mapping
      get().analyzeDataFlows();
    }
  },

  // Remove data mapping (parameter)
  removeDataMapping: (dataSource: DataSource, targetParam: { name: string; location: string }) => {
    const { arazzoSpec, selectedStepId } = get();
    if (!arazzoSpec || !selectedStepId) return;

    const workflows = [...arazzoSpec.workflows];
    let modified = false;

    for (const workflow of workflows) {
      // Find target step and remove parameter
      const targetStep = workflow.steps.find((s) => s.stepId === selectedStepId);
      if (targetStep && targetStep.parameters) {
        const paramIndex = targetStep.parameters.findIndex(
          (p) => p.name === targetParam.name && p.in === targetParam.location,
        );

        if (paramIndex !== -1) {
          targetStep.parameters.splice(paramIndex, 1);
          modified = true;
        }
      }
    }

    if (modified) {
      const newSpec = { ...arazzoSpec, workflows };
      get().setArazzoSpec(newSpec);
      // Re-analyze data flows after removing mapping
      get().analyzeDataFlows();
    }
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

    // Auto-generate outputs from response schema
    const outputs: Record<string, string> = {};
    if (operation.response_schema && operation.response_schema.properties.length > 0) {
      for (const property of operation.response_schema.properties) {
        outputs[property] = `$response.body.${property}`;
      }
    }

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
      outputs: Object.keys(outputs).length > 0 ? outputs : undefined,
    });

    const newSpec = {
      ...arazzoSpec,
      workflows,
    };

    get().setArazzoSpec(newSpec);
    // Analyze data flows after adding operation
    get().analyzeDataFlows();
  },

  // Analyze data flows and generate suggestions
  analyzeDataFlows: () => {
    const { arazzoSpec, operations, dismissedSuggestions } = get();
    if (!arazzoSpec || arazzoSpec.workflows.length === 0) {
      set({ dataFlowSuggestions: [] });
      return;
    }

    // Build operation map for quick lookup
    const operationMap = new Map<string, OperationInfo>();
    operations.forEach((op) => operationMap.set(op.operation_id, op));

    // Analyze each workflow
    const allSuggestions: DataFlowSuggestion[] = [];
    for (const workflow of arazzoSpec.workflows) {
      const suggestions = analyzeDataFlow(workflow.steps, operationMap);
      allSuggestions.push(...suggestions);
    }

    // Filter out dismissed suggestions
    const filteredSuggestions = allSuggestions.filter((suggestion) => {
      const key = getSuggestionKey(suggestion);
      return !dismissedSuggestions.has(key);
    });

    set({ dataFlowSuggestions: filteredSuggestions });
  },

  // Apply a data flow suggestion
  applySuggestion: (suggestion: DataFlowSuggestion) => {
    const { arazzoSpec } = get();
    if (!arazzoSpec) return;

    const workflows = [...arazzoSpec.workflows];
    let modified = false;

    for (const workflow of workflows) {
      const targetStep = workflow.steps.find((s) => s.stepId === suggestion.targetStep);
      if (!targetStep) continue;

      // Initialize parameters if not exists
      if (!targetStep.parameters) {
        targetStep.parameters = [];
      }

      // Check if parameter already exists
      const existingParam = targetStep.parameters.find(
        (p) => p.name === suggestion.targetParameter && p.in === suggestion.targetLocation,
      );

      if (!existingParam) {
        // Add new parameter with reference to source output
        targetStep.parameters.push({
          name: suggestion.targetParameter,
          in: suggestion.targetLocation,
          value: `$steps.${suggestion.sourceStep}.outputs.${suggestion.sourceOutput}`,
        });
        modified = true;
      }
    }

    if (modified) {
      const newSpec = { ...arazzoSpec, workflows };
      get().setArazzoSpec(newSpec);

      // Remove applied suggestion
      const { dataFlowSuggestions } = get();
      const updatedSuggestions = dataFlowSuggestions.filter((s) => s !== suggestion);
      set({ dataFlowSuggestions: updatedSuggestions });
    }
  },

  // Dismiss a suggestion
  dismissSuggestion: (suggestion: DataFlowSuggestion) => {
    const { dataFlowSuggestions, dismissedSuggestions } = get();
    const key = getSuggestionKey(suggestion);

    // Add to dismissed set
    const newDismissed = new Set(dismissedSuggestions);
    newDismissed.add(key);

    // Remove from current suggestions
    const updatedSuggestions = dataFlowSuggestions.filter((s) => s !== suggestion);

    set({
      dismissedSuggestions: newDismissed,
      dataFlowSuggestions: updatedSuggestions,
    });
  },

  // Reset editor
  reset: () => {
    set({
      arazzoSpec: initialArazzoSpec,
      yamlContent: yaml.dump(initialArazzoSpec),
      validationErrors: [],
      isValid: true,
      dataFlowSuggestions: [],
      dismissedSuggestions: new Set(),
      selectedOperation: null,
      selectedStepId: null,
      dataSourcePanelOpen: false,
      suggestionPanelOpen: false,
      error: null,
    });
  },
}));

// Helper function to create unique key for suggestion
function getSuggestionKey(suggestion: DataFlowSuggestion): string {
  return `${suggestion.sourceStep}:${suggestion.sourceOutput}:${suggestion.targetStep}:${suggestion.targetParameter}`;
}
