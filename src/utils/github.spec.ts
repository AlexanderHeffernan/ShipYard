import { describe, expect, it } from 'vitest';
import type { ConversationEntry, PullRequestCheck } from '../types/github';
import {
  authorInitials,
  checkOutcomeLabel,
  checkTone,
  conversationKindLabel,
  formatCheckDuration,
} from './github';
import { renderMarkdown } from './markdown';

const check = (overrides: Partial<PullRequestCheck> = {}): PullRequestCheck => ({
  id: 'check-1',
  name: 'unit tests',
  workflowName: 'CI',
  status: 'COMPLETED',
  conclusion: 'SUCCESS',
  startedAt: '2026-08-12T10:00:00Z',
  completedAt: '2026-08-12T10:01:30Z',
  url: null,
  ...overrides,
});

describe('GitHub review models', () => {
  it('maps check outcomes to stable UI labels and tones', () => {
    expect(checkTone(check())).toBe('success');
    expect(checkOutcomeLabel(check())).toBe('Passed');
    expect(checkTone(check({ conclusion: 'FAILURE' }))).toBe('danger');
    expect(checkOutcomeLabel(check({ conclusion: null, status: 'IN_PROGRESS' }))).toBe('In progress');
  });

  it('formats completed and in-progress check timing', () => {
    expect(formatCheckDuration(check())).toBe('1m 30s');
    expect(formatCheckDuration(check({ conclusion: null, completedAt: null }), Date.parse('2026-08-12T10:03:15Z'))).toBe('3m 15s elapsed');
  });

  it('labels discussion kinds and generates avatar fallbacks', () => {
    const review: ConversationEntry = {
      id: 'review-1',
      kind: 'review',
      author: { login: 'alex', name: 'Alex Heffernan', avatarUrl: null, profileUrl: null },
      body: '',
      timestamp: '',
      updatedAt: null,
      state: 'APPROVED',
      url: null,
      path: null,
      line: null,
    };
    expect(conversationKindLabel(review)).toBe('Approved');
    expect(authorInitials(review.author)).toBe('AH');
  });

  it('renders safe markdown without allowing raw HTML through', () => {
    const html = renderMarkdown('**Ready**\n\n`npm test`\n\n<script>alert(1)</script>');
    expect(html).toContain('<strong>Ready</strong>');
    expect(html).toContain('<code>npm test</code>');
    expect(html).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(html).not.toContain('<script>');
  });
});
