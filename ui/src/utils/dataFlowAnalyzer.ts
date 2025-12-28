import type { ArazzoStep, OperationInfo } from '../types/editor';

export interface DataFlowSuggestion {
  sourceStep: string;
  sourceOutput: string;
  targetStep: string;
  targetParameter: string;
  targetLocation: string; // "query", "header", "path", "body"
  confidence: 'high' | 'medium' | 'low';
  reason: string;
}

/**
 * Analyze data flow between steps and suggest parameter mappings
 * Analyzes all possible source-target step combinations (not just adjacent steps)
 */
export function analyzeDataFlow(
  steps: ArazzoStep[],
  operations: Map<string, OperationInfo>,
): DataFlowSuggestion[] {
  const suggestions: DataFlowSuggestion[] = [];

  // For each target step (starting from the second step)
  for (let targetIdx = 1; targetIdx < steps.length; targetIdx++) {
    const targetStep = steps[targetIdx];

    if (!targetStep.operationId) continue;

    const targetOperation = operations.get(targetStep.operationId);
    if (!targetOperation) continue;

    // Get existing parameters in target step to avoid duplicate suggestions
    const existingParams = new Set((targetStep.parameters || []).map((p) => `${p.name}:${p.in}`));

    // Check each parameter in target operation
    for (const param of targetOperation.parameters) {
      // Skip if parameter already exists in step
      if (existingParams.has(`${param.name}:${param.location}`)) {
        continue;
      }

      // Check all previous steps as potential sources
      for (let sourceIdx = 0; sourceIdx < targetIdx; sourceIdx++) {
        const sourceStep = steps[sourceIdx];

        if (!sourceStep.operationId) continue;

        // Get outputs from source step
        const outputs = sourceStep.outputs || {};
        const outputKeys = Object.keys(outputs);

        // Look for matching output names
        for (const outputKey of outputKeys) {
          const similarity = calculateSimilarity(outputKey, param.name);

          if (similarity > 0.8) {
            // High confidence match
            suggestions.push({
              sourceStep: sourceStep.stepId,
              sourceOutput: outputKey,
              targetStep: targetStep.stepId,
              targetParameter: param.name,
              targetLocation: param.location,
              confidence: 'high',
              reason: `Exact or near-exact name match: "${outputKey}" → "${param.name}"`,
            });
          } else if (similarity > 0.5) {
            // Medium confidence match
            suggestions.push({
              sourceStep: sourceStep.stepId,
              sourceOutput: outputKey,
              targetStep: targetStep.stepId,
              targetParameter: param.name,
              targetLocation: param.location,
              confidence: 'medium',
              reason: `Similar names: "${outputKey}" → "${param.name}"`,
            });
          }
        }

        // Special case: token/auth patterns
        if (isAuthRelated(param.name) && hasAuthOutput(outputKeys)) {
          const authOutput = outputKeys.find((k) => isAuthRelated(k));
          if (authOutput) {
            suggestions.push({
              sourceStep: sourceStep.stepId,
              sourceOutput: authOutput,
              targetStep: targetStep.stepId,
              targetParameter: param.name,
              targetLocation: param.location,
              confidence: 'high',
              reason: 'Authentication token detected',
            });
          }
        }

        // Special case: ID patterns
        if (isIdPattern(param.name) && hasIdOutput(outputKeys, param.name)) {
          const idOutput = outputKeys.find((k) => matchesIdPattern(k, param.name));
          if (idOutput) {
            suggestions.push({
              sourceStep: sourceStep.stepId,
              sourceOutput: idOutput,
              targetStep: targetStep.stepId,
              targetParameter: param.name,
              targetLocation: param.location,
              confidence: 'high',
              reason: 'ID parameter detected',
            });
          }
        }
      }
    }
  }

  // Remove duplicates
  return deduplicateSuggestions(suggestions);
}

/**
 * Calculate string similarity (0-1)
 */
function calculateSimilarity(str1: string, str2: string): number {
  const s1 = str1.toLowerCase();
  const s2 = str2.toLowerCase();

  if (s1 === s2) return 1.0;

  // Levenshtein distance
  const matrix: number[][] = [];
  const len1 = s1.length;
  const len2 = s2.length;

  for (let i = 0; i <= len1; i++) {
    matrix[i] = [i];
  }

  for (let j = 0; j <= len2; j++) {
    matrix[0][j] = j;
  }

  for (let i = 1; i <= len1; i++) {
    for (let j = 1; j <= len2; j++) {
      const cost = s1[i - 1] === s2[j - 1] ? 0 : 1;
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1,
        matrix[i][j - 1] + 1,
        matrix[i - 1][j - 1] + cost,
      );
    }
  }

  const distance = matrix[len1][len2];
  const maxLen = Math.max(len1, len2);
  return 1 - distance / maxLen;
}

/**
 * Check if parameter name is auth-related
 */
function isAuthRelated(name: string): boolean {
  const authPatterns = [
    /^auth/i,
    /^token/i,
    /^bearer/i,
    /^jwt/i,
    /^api[-_]?key/i,
    /^access[-_]?token/i,
    /^refresh[-_]?token/i,
  ];

  return authPatterns.some((pattern) => pattern.test(name));
}

/**
 * Check if outputs contain auth-related data
 */
function hasAuthOutput(outputKeys: string[]): boolean {
  return outputKeys.some((key) => isAuthRelated(key));
}

/**
 * Check if parameter name is an ID pattern
 */
function isIdPattern(name: string): boolean {
  return /\bid\b|_id$|Id$/i.test(name);
}

/**
 * Check if outputs contain matching ID
 */
function hasIdOutput(outputKeys: string[], paramName: string): boolean {
  return outputKeys.some((key) => matchesIdPattern(key, paramName));
}

/**
 * Check if output key matches ID parameter pattern
 */
function matchesIdPattern(outputKey: string, paramName: string): boolean {
  // e.g., "userId" matches "userId" or "id"
  // e.g., "id" matches "userId", "postId", etc.
  const outputLower = outputKey.toLowerCase();
  const paramLower = paramName.toLowerCase();

  if (outputLower === paramLower) return true;
  if (outputLower.endsWith('id') && paramLower.endsWith('id')) {
    // Extract prefix (e.g., "user" from "userId")
    const outputPrefix = outputLower.replace(/id$/, '');
    const paramPrefix = paramLower.replace(/id$/, '');
    return outputPrefix === paramPrefix || outputPrefix === '' || paramPrefix === '';
  }

  return false;
}

/**
 * Remove duplicate suggestions
 */
function deduplicateSuggestions(suggestions: DataFlowSuggestion[]): DataFlowSuggestion[] {
  const seen = new Set<string>();
  const result: DataFlowSuggestion[] = [];

  for (const suggestion of suggestions) {
    const key = `${suggestion.sourceStep}:${suggestion.sourceOutput}:${suggestion.targetStep}:${suggestion.targetParameter}`;
    if (!seen.has(key)) {
      seen.add(key);
      result.push(suggestion);
    }
  }

  // Sort by confidence
  return result.sort((a, b) => {
    const confidenceOrder = { high: 0, medium: 1, low: 2 };
    return confidenceOrder[a.confidence] - confidenceOrder[b.confidence];
  });
}
