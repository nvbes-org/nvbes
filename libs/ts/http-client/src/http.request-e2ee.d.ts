export declare function encryptRequestBody(input: {
  body: string;
  keyId: string;
  method: string;
  secret: string;
  url: string;
}): Promise<{
  body: ArrayBuffer;
  headers: Headers;
}>;
