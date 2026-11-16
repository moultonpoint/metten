import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { runInNewContext } from 'node:vm';
import { setImmediate } from 'node:timers/promises';
import { presentation, strategyFailure } from '../src/view.ts';

export async function harness(t, replies = {}) {
  const elements = new Map();
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      value: '', textContent: '', disabled: false, handlers: {}, children: [],
      addEventListener(event, handler) { this.handlers[event] = handler; },
      append(...items) { this.children.push(...items); },
      replaceChildren(...items) { this.children = items; }, focus() { this.focused = true; },
    });
    return elements.get(id);
  }
  const calls = [], pending = [], timers = [], unexpected = [];
  t.after(() => assert.deepEqual(unexpected, [], unexpected.join('\n')));
  let starting = true;
  function invoke(command, args) {
    const call = { command, args };
    calls.push(call);
    if (!['snapshot', 'strategies', 'control', 'credential'].includes(command)) {
      const message = `Unexpected native command: ${command}`;
      unexpected.push(message);
      return Promise.reject(new Error(message));
    }
    if (Object.hasOwn(replies, command)) return Promise.resolve().then(() => replies[command](args));
    if (command === 'snapshot') return Promise.resolve({
      running: false, pending: 0, recording_error: null,
      history: [{ status: 'stopped', tick: 0, reason: 'test', recorded_at: 'test' }], writer_settings: {},
    });
    if (command === 'strategies' && starting && args.operation === 'list') {
      return Promise.resolve({ kind: 'versions', versions: [], next_cursor: null });
    }
    if (command === 'control') return Promise.resolve();
    return new Promise((resolve, reject) => { pending.push(Object.assign(call, { resolve, reject })); });
  }
  const source = stripTypeScriptTypes(await readFile(new URL('../src/main.ts', import.meta.url), 'utf8')).replace(/^import .*;\n/gm, '');
  runInNewContext(source, {
    document: { getElementById: element, createElement: tag => element(Symbol(tag)) },
    window: { setTimeout(callback, delay) { timers.push({ callback, delay }); } },
    TextEncoder, presentation, strategyFailure, invoke,
  });
  starting = false;
  await setImmediate();
  return {
    elements, element, calls, pending, timers, invoke, flush: setImmediate,
    click: id => element(id).handlers.click(),
    edit(text) { element('strategy-json').value = text; element('strategy-json').handlers.input(); },
  };
}
