import { describe, it, expect } from 'vitest';
import { analyzeDataFlow } from '../dataFlowAnalyzer';
import type { ArazzoStep, OperationInfo } from '../../types/editor';

describe('dataFlowAnalyzer', () => {
  // Mock operation data
  const mockOperationMap = new Map<string, OperationInfo>([
    [
      'loginUser',
      {
        operation_id: 'loginUser',
        method: 'POST',
        path: '/login',
        summary: 'User login',
        description: 'Authenticate a user and return a token',
        parameters: [],
        request_body: {
          required: true,
          content_types: ['application/json'],
          description: undefined,
        },
        response_codes: ['200', '401'],
        tags: [],
      },
    ],
    [
      'getProfile',
      {
        operation_id: 'getProfile',
        method: 'GET',
        path: '/profile',
        summary: 'Get user profile',
        description: "Retrieve the authenticated user's profile",
        parameters: [
          {
            name: 'Authorization',
            location: 'header',
            required: true,
            description: 'Bearer token',
            schema_type: 'string',
          },
        ],
        request_body: undefined,
        response_codes: ['200', '401'],
        tags: [],
      },
    ],
  ]);

  describe('analyzeDataFlow', () => {
    it('should detect auth token flow from login to getProfile', () => {
      const steps: ArazzoStep[] = [
        {
          stepId: 'login',
          description: 'Login user',
          operationId: 'loginUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
          outputs: {
            token: '$response.body.token',
            userId: '$response.body.user.id',
          },
        },
        {
          stepId: 'getProfile',
          description: 'Get user profile',
          operationId: 'getProfile',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
        },
      ];

      const suggestions = analyzeDataFlow(steps, mockOperationMap);

      // Should suggest mapping login.token to getProfile.Authorization
      expect(suggestions.length).toBeGreaterThan(0);

      const tokenSuggestion = suggestions.find(
        (s) =>
          s.sourceStep === 'login' &&
          s.sourceOutput === 'token' &&
          s.targetStep === 'getProfile' &&
          s.targetParameter === 'Authorization',
      );

      expect(tokenSuggestion).toBeDefined();
      expect(tokenSuggestion?.confidence).toBe('high');
      expect(tokenSuggestion?.targetLocation).toBe('header');
      expect(tokenSuggestion?.reason).toContain('Authentication');
    });

    it('should not suggest mappings that already exist', () => {
      const steps: ArazzoStep[] = [
        {
          stepId: 'login',
          description: 'Login user',
          operationId: 'loginUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
          outputs: {
            token: '$response.body.token',
          },
        },
        {
          stepId: 'getProfile',
          description: 'Get user profile',
          operationId: 'getProfile',
          parameters: [
            {
              name: 'Authorization',
              in: 'header',
              value: '$steps.login.outputs.token',
            },
          ],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
        },
      ];

      const suggestions = analyzeDataFlow(steps, mockOperationMap);

      // Should not suggest Authorization since it's already mapped
      const tokenSuggestion = suggestions.find(
        (s) =>
          s.sourceStep === 'login' &&
          s.sourceOutput === 'token' &&
          s.targetStep === 'getProfile' &&
          s.targetParameter === 'Authorization',
      );

      expect(tokenSuggestion).toBeUndefined();
    });

    it('should detect ID pattern mappings', () => {
      const operationMap = new Map<string, OperationInfo>([
        [
          'registerUser',
          {
            operation_id: 'registerUser',
            method: 'POST',
            path: '/register',
            summary: 'Register user',
            description: '',
            parameters: [],
            request_body: {
              required: true,
              content_types: ['application/json'],
              description: undefined,
            },
            response_codes: ['201'],
            tags: [],
          },
        ],
        [
          'getUser',
          {
            operation_id: 'getUser',
            method: 'GET',
            path: '/users/{userId}',
            summary: 'Get user by ID',
            description: '',
            parameters: [
              {
                name: 'userId',
                location: 'path',
                required: true,
                description: 'User ID',
                schema_type: 'string',
              },
            ],
            request_body: undefined,
            response_codes: ['200'],
            tags: [],
          },
        ],
      ]);

      const steps: ArazzoStep[] = [
        {
          stepId: 'register',
          description: 'Register user',
          operationId: 'registerUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 201,
            },
          ],
          outputs: {
            userId: '$response.body.id',
          },
        },
        {
          stepId: 'getUser',
          description: 'Get user',
          operationId: 'getUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
        },
      ];

      const suggestions = analyzeDataFlow(steps, operationMap);

      // Should suggest mapping register.userId to getUser.userId
      const idSuggestion = suggestions.find(
        (s) =>
          s.sourceStep === 'register' &&
          s.sourceOutput === 'userId' &&
          s.targetStep === 'getUser' &&
          s.targetParameter === 'userId',
      );

      expect(idSuggestion).toBeDefined();
      expect(idSuggestion?.confidence).toBe('high');
      expect(idSuggestion?.targetLocation).toBe('path');
    });

    it('should return empty suggestions for single step workflow', () => {
      const steps: ArazzoStep[] = [
        {
          stepId: 'login',
          description: 'Login user',
          operationId: 'loginUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
          outputs: {
            token: '$response.body.token',
          },
        },
      ];

      const suggestions = analyzeDataFlow(steps, mockOperationMap);

      expect(suggestions).toEqual([]);
    });

    it('should handle steps without outputs', () => {
      const steps: ArazzoStep[] = [
        {
          stepId: 'login',
          description: 'Login user',
          operationId: 'loginUser',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
          // No outputs
        },
        {
          stepId: 'getProfile',
          description: 'Get user profile',
          operationId: 'getProfile',
          parameters: [],
          successCriteria: [
            {
              context: '$statusCode',
              condition: '==',
              value: 200,
            },
          ],
        },
      ];

      const suggestions = analyzeDataFlow(steps, mockOperationMap);

      expect(suggestions).toEqual([]);
    });
  });
});
