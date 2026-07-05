import type { operations } from '@nvbes/internal-admin-sdk-core/src/types';

type JsonContent<TResponse> = TResponse extends {
  content: { 'application/json': infer TJson };
}
  ? TJson
  : never;

export type OperationPath<TOperation extends keyof operations> =
  operations[TOperation]['parameters'] extends { path: infer TPath } ? TPath : never;

export type OperationBody<TOperation extends keyof operations> = operations[TOperation] extends {
  requestBody: { content: { 'application/json': infer TBody } };
}
  ? TBody
  : never;

export type OperationOk<TOperation extends keyof operations> =
  operations[TOperation]['responses'] extends { 200: infer TResponse }
    ? JsonContent<TResponse>
    : never;
