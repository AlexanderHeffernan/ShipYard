import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WorkItemConversation from './WorkItemConversation.vue';
import type { PullRequestConversation } from '../../types/github';
import type { Project, WorkItem } from '../../types/projects';
import { getPullRequestConversation, postPullRequestComment } from '../../services/github';

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../../services/github', () => ({
  getPullRequestConversation: vi.fn(),
  postPullRequestComment: vi.fn(),
}));

const getConversation = vi.mocked(getPullRequestConversation);
const postComment = vi.mocked(postPullRequestComment);

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
    checksReported: true,
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

const conversation: PullRequestConversation = {
  viewerLogin: 'alex',
  entries: [{
    id: 'comment-1',
    kind: 'comment',
    author: { login: 'reviewer', name: 'Reviewer', avatarUrl: null, profileUrl: null },
    body: 'Looks **good** to me.',
    timestamp: '2026-08-12T10:00:00Z',
    updatedAt: null,
    state: null,
    url: null,
    path: null,
    line: null,
  }],
};

describe('WorkItemConversation', () => {
  beforeEach(() => vi.resetAllMocks());

  it('renders markdown discussion and posts a validated top-level comment', async () => {
    getConversation.mockResolvedValue(conversation);
    postComment.mockResolvedValue({
      id: 'comment-2',
      kind: 'comment',
      author: { login: 'alex', name: 'Alex', avatarUrl: null, profileUrl: null },
      body: 'Thanks for reviewing.',
      timestamp: '2026-08-12T10:05:00Z',
      updatedAt: null,
      state: null,
      url: null,
      path: null,
      line: null,
    });
    const wrapper = mount(WorkItemConversation, { props: { project, workItem } });
    await flushPromises();

    expect(wrapper.find('.conversation-entry__body strong').text()).toBe('good');
    await wrapper.get('textarea').setValue('Thanks for reviewing.');
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    expect(postComment).toHaveBeenCalledWith({
      repository: project.githubRepository,
      number: 42,
      body: 'Thanks for reviewing.',
    });
    expect(wrapper.text()).toContain('Posted');
    expect(wrapper.text()).toContain('Thanks for reviewing.');
  });

  it('validates empty comments before calling GitHub', async () => {
    getConversation.mockResolvedValue({ ...conversation, entries: [] });
    const wrapper = mount(WorkItemConversation, { props: { project, workItem } });
    await flushPromises();
    await wrapper.get('form').trigger('submit');

    expect(wrapper.text()).toContain('Write a comment before posting.');
    expect(postComment).not.toHaveBeenCalled();
  });
});
