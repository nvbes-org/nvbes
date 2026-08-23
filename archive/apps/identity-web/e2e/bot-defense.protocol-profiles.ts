import { invalidPowSolution, requiredString, type JsonObject } from './bot-defense.support';

export interface ProtocolAttack {
  name: string;
  expectedCode: string;
  mutate: (body: JsonObject) => JsonObject | Promise<JsonObject>;
}

export const BOT_GUARD_ATTACKS: readonly ProtocolAttack[] = [
  {
    name: 'a clicked decoy link',
    expectedCode: 'decoy_link_clicked',
    mutate: (body) => ({ ...body, decoy_link_clicked: true }),
  },
  {
    name: 'a filled honeypot field',
    expectedCode: 'bot_honeypot_triggered',
    mutate: (body) => ({
      ...body,
      bot_guard: { website: 'https://spam.example', form_ts: null, js_sig: null },
    }),
  },
  {
    name: 'an impossibly fast form submission',
    expectedCode: 'bot_time_lock_fast',
    mutate: (body) => ({
      ...body,
      bot_guard: { website: '', form_ts: Date.now(), js_sig: 'invalid' },
    }),
  },
  {
    name: 'a form timestamp in the future',
    expectedCode: 'bot_time_lock_future',
    mutate: (body) => ({
      ...body,
      bot_guard: { website: '', form_ts: Date.now() + 60_000, js_sig: 'invalid' },
    }),
  },
  {
    name: 'an expired form timestamp',
    expectedCode: 'bot_time_lock_expired',
    mutate: (body) => ({
      ...body,
      bot_guard: { website: '', form_ts: Date.now() - 601_000, js_sig: 'invalid' },
    }),
  },
  {
    name: 'a forged JavaScript signature',
    expectedCode: 'bot_js_signature_invalid',
    mutate: (body) => ({
      ...body,
      bot_guard: { website: '', form_ts: Date.now() - 2_000, js_sig: 'deadbeef' },
    }),
  },
] as const;

export const POW_ATTACKS: readonly ProtocolAttack[] = [
  {
    name: 'an empty proof',
    expectedCode: 'pow_missing',
    mutate: (body) => ({ ...body, pow_nonce: '', pow_solution: '' }),
  },
  {
    name: 'an unknown nonce',
    expectedCode: 'pow_invalid_nonce',
    mutate: (body) => ({ ...body, pow_nonce: `unknown:${crypto.randomUUID()}` }),
  },
  {
    name: 'an invalid solution',
    expectedCode: 'pow_solution_invalid',
    mutate: async (body) => ({
      ...body,
      pow_solution: await invalidPowSolution(requiredString(body.pow_nonce, 'pow_nonce')),
    }),
  },
] as const;
