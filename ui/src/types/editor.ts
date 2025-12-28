// Editor-related types

export interface OperationInfo {
  operation_id: string;
  method: string;
  path: string;
  summary?: string;
  description?: string;
  parameters: ParameterInfo[];
  request_body?: RequestBodyInfo;
  tags: string[];
  response_codes: string[]; // HTTP status codes like "200", "201", "404"
  response_schema?: ResponseSchemaInfo; // Response schema for 2xx responses
}

export interface ResponseSchemaInfo {
  status_code: string;
  properties: string[]; // Top-level property names in response body
}

export interface ParameterInfo {
  name: string;
  location: string;
  required: boolean;
  schema_type?: string;
  description?: string;
}

export interface RequestBodyInfo {
  required: boolean;
  content_types: string[];
  description?: string;
}

export interface ValidationError {
  message: string;
  line?: number;
  column?: number;
}

export interface ValidationResponse {
  valid: boolean;
  errors: ValidationError[];
}

// Arazzo Spec types (simplified for editor)
export interface ArazzoWorkflow {
  workflowId: string;
  summary?: string;
  description?: string;
  inputs?: Record<string, unknown>;
  steps: ArazzoStep[];
  outputs?: Record<string, unknown>;
}

export interface ArazzoStep {
  stepId: string;
  description?: string;
  operationId?: string;
  operationPath?: string;
  parameters?: ArazzoParameter[];
  requestBody?: {
    contentType?: string;
    payload?: Record<string, unknown>;
  };
  successCriteria?: ArazzoSuccessCriteria[];
  outputs?: Record<string, string>;
}

export interface ArazzoParameter {
  name: string;
  in: string;
  value: unknown;
}

export interface ArazzoSuccessCriteria {
  context: string;
  condition: string;
  value: unknown;
}

export interface ArazzoSpec {
  arazzo: string;
  info: {
    title: string;
    description?: string;
    version: string;
  };
  sourceDescriptions?: Array<{
    name: string;
    url: string;
    type: string;
  }>;
  workflows: ArazzoWorkflow[];
}
