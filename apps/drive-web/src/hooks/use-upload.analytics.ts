function bytesBucket(bytes: number): string {
  if (bytes < 1024 * 1024) return '<1mb';
  if (bytes < 10 * 1024 * 1024) return '1-10mb';
  if (bytes < 100 * 1024 * 1024) return '10-100mb';
  if (bytes < 1024 * 1024 * 1024) return '100mb-1gb';
  return '1gb+';
}

export function uploadTrackingPayload(totalBytes: number) {
  return {
    total_bytes_bucket: bytesBucket(totalBytes),
  };
}
