import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WorkItemChecks from './WorkItemChecks.vue';
import type { PullRequestChecks } from '../../types/github';
import type { Project, WorkItem } from '../../types/projects';
import { getPullRequestChecks } from '../../services/github';

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../../services/github', () => ({
  getPullRequestChecks: vi.fn(),
}));

const getChecks = vi.mocked(getPullRequestChecks);

const project = {
  id: 'project-1',
  name: 'Shipyard',
  path: '/tmp/shipyard',
  defaultBranch: 'main',
  githubRepository: 'alexanderheffernan/shipyard',
  githubError: null,
  color: '#fb771f',
  workItems: [],
} as Project;

const workItem = {
  id: 'project-1::pull-request::42',
  projectId: project.id,
  branch: null,
  worktreePath: null,
  headSha: 'abc1234',
  lastCommitSubject: 'Add review surface',
  status: 'ready',
  pullRequest: {
    number: 42,
    title: 'Add review surface',
    url: 'https://github.com/alexanderheffernan/shipyard/pull/42',
    draft: false,
    mergeable: true,
    mergeState: 'ready',
    headBranch: 'feature/review',
    baseBranch: 'main',
    headSha: 'abc1234',
    localCommits: 0,
    remoteCommits: 0,
  },
  completed: false,
  additions: 4,
  deletions: 1,
  changedFiles: 1,
  ahead: 0,
  behind: 0,
  updatedAt: 1,
} as WorkItem;

const passingChecks: PullRequestChecks = {
  overallState: 'success',
  total: 1,
  passed: 1,
  failed: 0,
  pending: 0,
  neutral: 0,
  lastUpdatedAt: '2026-08-12T10:01:30Z',
  checks: [{
    id: 'build-1',
    name: 'Build',
    workflowName: 'CI',
    status: 'COMPLETED',
    conclusion: 'SUCCESS',
    startedAt: '2026-08-12T10:00:00Z',
    completedAt: '2026-08-12T10:01:30Z',
    url: 'https://github.com/alexanderheffernan/shipyard/actions/runs/1',
  }],
};

describe('WorkItemChecks', () => {
  beforeEach(() => vi.resetAllMocks());

  it('shows readiness and individual check details', async () => {
    getChecks.mockResolvedValue(passingChecks);
    const wrapper = mount(WorkItemChecks, { props: { project, workItem } });
    await flushPromises();

    expect(wrapper.text()).toContain('Ready to merge');
    expect(wrapper.text()).toContain('Build');
    expect(wrapper.text()).toContain('Passed');
    expect(wrapper.findAll('.check-row')).toHaveLength(1);
  });

  it('shows a retryable error when the GitHub service is unavailable', async () => {
    getChecks.mockRejectedValueOnce(new Error('GitHub API rate limit reached'));
    const wrapper = mount(WorkItemChecks, { props: { project, workItem } });
    await flushPromises();

    expect(wrapper.text()).toContain('Couldn’t load checks');
    expect(wrapper.text()).toContain('GitHub API rate limit reached');
    getChecks.mockResolvedValueOnce(passingChecks);
    await wrapper.get('button').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Ready to merge');
  });

  it('qualifies mergeability when GitHub reports no visible checks', async () => {
    getChecks.mockResolvedValue({
      overallState: 'unknown',
      total: 0,
      passed: 0,
      failed: 0,
      pending: 0,
      neutral: 0,
      lastUpdatedAt: null,
      checks: [],
    });
    const wrapper = mount(WorkItemChecks, { props: { project, workItem } });
    await flushPromises();

    expect(wrapper.text()).toContain('Mergeable, but checks are not visible');
    expect(wrapper.text()).toContain('Verify required checks on GitHub before merging.');
  });
});
