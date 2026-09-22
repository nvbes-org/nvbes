#!/usr/bin/env node

import { appendFileSync, readFileSync } from 'node:fs';
import { evaluatePullRequestCacheTrust } from './authorize-pr-cache.core.mjs';

function requiredEnvironment(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} must be set`);
  return value;
}

const ALLOWED_PR_RESOURCES = new Set(['commits', 'files']);
const REPOSITORY_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;

function validatedApiBaseUrl(rawApiUrl) {
  let parsed;
  try {
    parsed = new URL(rawApiUrl);
  } catch {
    throw new Error('GITHUB_API_URL is not a valid URL');
  }
  if (parsed.protocol !== 'https:') throw new Error('GITHUB_API_URL must use https');
  if (parsed.username || parsed.password)
    throw new Error('GITHUB_API_URL must not embed credentials');
  return parsed.toString().replace(/\/+$/u, '');
}

function validatedRepository(rawRepository) {
  if (!REPOSITORY_PATTERN.test(rawRepository)) throw new Error('GITHUB_REPOSITORY is invalid');
  return rawRepository;
}

function validatedResource(resource) {
  if (!ALLOWED_PR_RESOURCES.has(resource))
    throw new Error(`Unsupported pull request resource: ${resource}`);
  return resource;
}

async function listPullRequestResource(apiUrl, repository, number, resource) {
  const token = requiredEnvironment('GITHUB_TOKEN');
  const safeBase = validatedApiBaseUrl(apiUrl);
  const safeRepository = validatedRepository(repository);
  const safeResource = validatedResource(resource);
  if (!Number.isInteger(number) || number <= 0) throw new Error('Pull request number is invalid');
  const values = [];
  for (let page = 1; page <= 10; page += 1) {
    const endpoint = new URL(`${safeBase}/repos/${safeRepository}/pulls/${number}/${safeResource}`);
    endpoint.searchParams.set('per_page', '100');
    endpoint.searchParams.set('page', String(page));
    const response = await fetch(endpoint, {
      headers: {
        Accept: 'application/vnd.github+json',
        Authorization: `Bearer ${token}`,
        'X-GitHub-Api-Version': '2022-11-28',
      },
    });
    if (!response.ok) {
      throw new Error(`GitHub ${resource} query failed with ${response.status}`);
    }
    const pageValues = await response.json();
    if (!Array.isArray(pageValues)) throw new Error(`Invalid ${resource} response`);
    values.push(...pageValues);
    if (pageValues.length < 100) return values;
  }
  throw new Error(`Pull request ${resource} exceed the authorization limit`);
}

const event = JSON.parse(readFileSync(requiredEnvironment('GITHUB_EVENT_PATH'), 'utf8'));
const repository = requiredEnvironment('GITHUB_REPOSITORY');
const apiUrl = requiredEnvironment('GITHUB_API_URL');
const output = requiredEnvironment('GITHUB_OUTPUT');
let result = { trusted: false, reasons: ['authorization failed closed'] };

try {
  const number = event.pull_request?.number;
  if (!Number.isInteger(number)) throw new Error('Pull request number is missing');
  const [commits, files] = await Promise.all([
    listPullRequestResource(apiUrl, repository, number, 'commits'),
    listPullRequestResource(apiUrl, repository, number, 'files'),
  ]);
  result = evaluatePullRequestCacheTrust(event, commits, files);
} catch (error) {
  result = { trusted: false, reasons: [error.message] };
}

appendFileSync(output, `trusted=${result.trusted}\n`);
console.log(
  result.trusted
    ? 'Internal pull request cache access authorized'
    : `Central cache disabled: ${result.reasons.join('; ')}`,
);
