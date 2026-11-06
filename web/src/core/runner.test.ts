import { describe, expect, it } from 'vitest'

import {
  canStart,
  canStop,
  createdAgo,
  groupByOwner,
  healthOf,
  isFiltering,
  labelsOf,
  ownersOf,
  repositoriesOf,
  type Runner,
  statusText,
  summarize,
  toQuery,
} from './runner'

function runner(overrides: Partial<Runner> = {}): Runner {
  return {
    id: '67e55044-10b1-426f-9247-bb680e5fe0c8',
    forge: 'github',
    url: 'https://github.com/otsobide/mindweaver',
    owner: 'otsobide',
    repository: 'mindweaver',
    scope: 'repository',
    name: 'builder',
    labels: ['gpu'],
    lifecycle: 'persistent',
    image: 'ghcr.io/actions/actions-runner:latest',
    status: 'running',
    exit_code: null,
    created_at: 1_700_000_000,
    ...overrides,
  }
}

describe('healthOf', () => {
  it('takes a running runner for up', () => {
    expect(healthOf(runner())).toBe('up')
  })

  it('takes a runner being restarted for trouble', () => {
    expect(healthOf(runner({ status: 'restarting', exit_code: 1 }))).toBe('trouble')
  })

  it('takes a runner that exited cleanly for idle and one that failed for down', () => {
    expect(healthOf(runner({ status: 'exited', exit_code: 0 }))).toBe('idle')
    expect(healthOf(runner({ status: 'exited', exit_code: 1 }))).toBe('down')
    expect(healthOf(runner({ status: 'dead' }))).toBe('down')
  })
})

describe('statusText', () => {
  it('says why a runner is being restarted', () => {
    expect(statusText(runner({ status: 'restarting', exit_code: 1 }))).toBe(
      'Restarting after exiting with 1',
    )
  })

  it('tells a failed runner from a stopped one', () => {
    expect(statusText(runner({ status: 'exited', exit_code: 1 }))).toBe('Failed, exited with 1')
    expect(statusText(runner({ status: 'exited', exit_code: 0 }))).toBe('Stopped')
  })

  it('says an ephemeral runner that exited cleanly is done with its job', () => {
    expect(statusText(runner({ status: 'exited', exit_code: 0, lifecycle: 'ephemeral' }))).toBe(
      'Done with its job',
    )
  })
})

describe('canStart and canStop', () => {
  it('offers to stop a running runner, and to start a stopped one', () => {
    expect([canStart(runner()), canStop(runner())]).toEqual([false, true])
    const stopped = runner({ status: 'exited', exit_code: 0 })
    expect([canStart(stopped), canStop(stopped)]).toEqual([true, false])
  })

  it('offers to stop a runner caught restarting', () => {
    expect(canStop(runner({ status: 'restarting' }))).toBe(true)
  })
})

describe('groupByOwner', () => {
  it('groups runners by owner, in alphabetical order, whatever the case', () => {
    const groups = groupByOwner([
      runner({ owner: 'zeta', url: 'https://github.com/zeta/app', name: 'z' }),
      runner({ owner: 'otsobide', name: 'a' }),
      runner({ owner: 'Otsobide', url: 'https://github.com/Otsobide', repository: null, name: 'b' }),
    ])

    expect(groups.map((group) => [group.owner, group.runners.map((r) => r.name)])).toEqual([
      ['otsobide', ['a', 'b']],
      ['zeta', ['z']],
    ])
  })

  it('keeps owners of the same name on different forges apart', () => {
    const groups = groupByOwner([
      runner(),
      runner({ url: 'http://gitea.lan:3000/otsobide/mindweaver' }),
    ])

    expect(groups.map((group) => group.host)).toEqual(['gitea.lan:3000', 'github.com'])
  })
})

describe('summarize', () => {
  it('counts runners by health', () => {
    expect(
      summarize([
        runner(),
        runner(),
        runner({ status: 'restarting' }),
        runner({ status: 'exited', exit_code: 1 }),
      ]),
    ).toEqual({ up: 2, trouble: 1, down: 1, idle: 0 })
  })
})

describe('the options filters offer', () => {
  const runners = [
    runner({ owner: 'otsobide', repository: 'mindweaver', labels: ['gpu', 'large'] }),
    runner({ owner: 'Otsobide', repository: null, labels: ['GPU'] }),
    runner({ owner: 'acme', repository: 'site', labels: [] }),
  ]

  it('lists each owner once, in alphabetical order', () => {
    expect(ownersOf(runners)).toEqual(['acme', 'otsobide'])
  })

  it('lists the repositories of an owner, or of every owner', () => {
    expect(repositoriesOf(runners)).toEqual(['mindweaver', 'site'])
    expect(repositoriesOf(runners, 'OTSOBIDE')).toEqual(['mindweaver'])
  })

  it('lists each label once, whatever its case', () => {
    expect(labelsOf(runners)).toEqual(['gpu', 'large'])
  })
})

describe('toQuery and isFiltering', () => {
  it('leaves empty filters out of the query', () => {
    const query = toQuery({ owner: 'otsobide', status: '', q: '  build ' })

    expect(query.toString()).toBe('owner=otsobide&q=build')
  })

  it('tells filters that narrow from filters that do not', () => {
    expect(isFiltering({ owner: '', q: ' ' })).toBe(false)
    expect(isFiltering({ status: 'running' })).toBe(true)
  })
})

describe('createdAgo', () => {
  const now = new Date(1_700_000_000_000)

  it('says how long ago a runner was created', () => {
    expect(createdAgo(runner({ created_at: 1_700_000_000 - 30 }), now)).toBe('just now')
    expect(createdAgo(runner({ created_at: 1_700_000_000 - 5 * 60 }), now)).toBe('5 minutes ago')
    expect(createdAgo(runner({ created_at: 1_700_000_000 - 3 * 3600 }), now)).toBe('3 hours ago')
    expect(createdAgo(runner({ created_at: 1_700_000_000 - 86_400 }), now)).toBe('yesterday')
  })
})
