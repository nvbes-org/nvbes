import { text } from './hosted.transport';

export function credentialLabel(value: unknown): string {
  const label = text(value, 256);
  // Match Rust's char count (Unicode scalar values), not grapheme clusters.
  if (Array.from(label).length > 128 || /\p{Cc}/u.test(label))
    throw new Error('Invalid Identity credential label.');
  return label;
}

export function credentialId(value: unknown): string {
  const id = text(value, 36);
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(id))
    throw new Error('Invalid Identity credential ID.');
  return id;
}
