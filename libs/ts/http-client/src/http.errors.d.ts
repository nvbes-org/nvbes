import { z } from 'zod';
import type { HttpRecovery } from './http.types';
export declare class HttpError extends Error {
    readonly status: number;
    readonly statusText: string;
    readonly body: unknown;
    readonly recovery: HttpRecovery | undefined;
    readonly requestId: string | undefined;
    constructor(message: string, response: Response, body: unknown, requestId?: string);
}
export declare class DtoValidationError extends Error {
    readonly cause: z.ZodError;
    constructor(cause: z.ZodError);
}
export declare function readResponseBody(response: Response): Promise<unknown>;
export declare function readErrorMessage(response: Response, body: unknown): string;
export declare function readRequestId(response: Response, body: unknown): string | undefined;
