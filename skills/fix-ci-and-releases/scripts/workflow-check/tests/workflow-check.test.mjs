import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkWorkflow } from '../workflow-check.mjs';

const ids = (text) => checkWorkflow(text).map((f) => `${f.id}:${f.severity}`).sort();

test('a pinned, least-privilege workflow is clean', () => {
  assert.deepEqual(ids(`on: pull_request
permissions:
  contents: read
jobs:
  t:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - uses: ./local-action
      - name: Check title
        env:
          TITLE: \${{ github.event.pull_request.title }}
        run: |
          echo "$TITLE"
          echo done
      - run: cargo test --locked
`), []);
});

test('unpinned actions, injection and missing permissions are caught', () => {
  assert.deepEqual(ids(`on: pull_request
jobs:
  t:
    steps:
      - uses: actions/checkout@v7
      - uses: someone/tool@main
      - uses: docker://alpine:3
      - run: echo "\${{ github.event.pull_request.title }}"
      - run: |
          git checkout \${{ github.head_ref }}
          echo \${{ inputs.level }}
        continue-on-error: true
`), ['W1-pin:FAIL', 'W1-pin:FAIL', 'W1-pin:WARN', 'W2-permissions:WARN', 'W3-injection:FAIL', 'W3-injection:FAIL', 'W3-injection:WARN', 'W5-continue:WARN']);
});

test('pull_request_target that checks out the PR head fails', () => {
  const text = `on:
  pull_request_target:
permissions: write-all
jobs:
  t:
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: \${{ github.event.pull_request.head.sha }}
`;
  assert.deepEqual(ids(text), ['W2-permissions:FAIL', 'W4-privileged-trigger:FAIL']);
});

test('pull_request_target alone is a warning', () => {
  assert.deepEqual(ids(`on: [pull_request_target]\npermissions: {}\njobs: {}\n`), ['W4-privileged-trigger:WARN']);
});

test('expressions outside run: are not injection', () => {
  assert.deepEqual(ids(`on: push\npermissions: {}\njobs:\n  t:\n    if: \${{ github.event.head_commit.message != 'x' }}\n    steps:\n      - run: echo ok\n`), []);
});
