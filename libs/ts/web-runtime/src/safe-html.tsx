import type { ComponentProps } from 'react';
import { createTrustedHtml } from './trusted-types';

declare const safeHtmlBrand: unique symbol;

export type SafeHtml = string & {
  readonly [safeHtmlBrand]: true;
};

export function sanitizeHtml(value: string): SafeHtml {
  return stripUnsafeHtml(value) as SafeHtml;
}

export function trustSafeHtml(value: string, reason: string): SafeHtml {
  if (reason.trim().length < 12) {
    throw new Error('Trusted HTML requires a review reason.');
  }
  return value as SafeHtml;
}

export function safeHtmlToString(value: SafeHtml): string {
  return value;
}

export function VerifiedHtml({
  html,
  ...props
}: Omit<ComponentProps<'div'>, 'children' | 'dangerouslySetInnerHTML'> & {
  html: SafeHtml;
}) {
  return <div {...props} dangerouslySetInnerHTML={{ __html: toTrustedHtml(html) }} />;
}

function toTrustedHtml(value: SafeHtml): string | TrustedHTML {
  return createTrustedHtml(value);
}

const BLOCKED_CONTENT_TAGS =
  /<\s*(script|style|iframe|object|embed|svg|math|template)\b[^>]*>[\s\S]*?<\s*\/\s*\1\s*>/giu;
const BLOCKED_TAGS =
  /<\s*\/?\s*(script|style|iframe|object|embed|svg|math|template|base|link|meta)\b[^>]*>/giu;
const EVENT_HANDLER_ATTRIBUTES = /\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)/giu;
const DANGEROUS_ATTRIBUTES =
  /\s+(style|srcdoc|formaction|action|manifest|xmlns(?::[a-z]+)?)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)/giu;
const UNSAFE_URL_ATTRIBUTES =
  /\s+(href|src|xlink:href|poster)\s*=\s*(["'])\s*(?:javascript|vbscript|data(?!:image\/(?:png|jpeg|gif|webp|avif);base64,))[^"']*\2/giu;
const HTML_COMMENTS = /<!--[\s\S]*?-->/gu;

function stripUnsafeHtml(value: string): string {
  return value
    .replace(HTML_COMMENTS, '')
    .replace(BLOCKED_CONTENT_TAGS, '')
    .replace(BLOCKED_TAGS, '')
    .replace(EVENT_HANDLER_ATTRIBUTES, '')
    .replace(DANGEROUS_ATTRIBUTES, '')
    .replace(UNSAFE_URL_ATTRIBUTES, '');
}
