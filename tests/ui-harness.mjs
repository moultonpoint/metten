import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { runInNewContext } from 'node:vm';
import { setImmediate } from 'node:timers/promises';
import { presentation, strategyFailure, sourceFailure, sourcePresentation, paperMoney, paperReason, paperReceipt, paperFailure, paperChecking, paperRecovery } from '../src/view.ts';

export async function harness(t, replies = {}) {
  const elements = new Map();
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      value: '', textContent: '', disabled: false, handlers: {}, children: [], attributes: {},
      setAttribute(name, value) { this.attributes[name] = value; }, getAttribute(name) { return this.attributes[name] ?? null; },
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
    if (!['snapshot', 'strategies', 'control', 'credential', 'paper'].includes(command)) {
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
    if (command === 'paper' && args.operation === 'snapshot') return Promise.resolve({ kind: 'snapshot', run: null, effective_paused: false, effective_generation: '0', origin: 'synthetic_controlled', clock_multiplier: '20' });
    if (command === 'paper' && args.operation === 'history') return Promise.resolve({ kind: 'history', page: { markets: [], next_cursor: null } });
    return new Promise((resolve, reject) => { pending.push(Object.assign(call, { resolve, reject })); });
  }
  const source = stripTypeScriptTypes(await readFile(new URL('../src/main.ts', import.meta.url), 'utf8')).replace(/^import .*;\n/gm, '');
  const document = { hidden: false, handlers: {}, getElementById: element, createElement: tag => element(Symbol(tag)), addEventListener(event, handler) { this.handlers[event] = handler; } };
  const learningExample = await readFile(new URL('../examples/learning.json', import.meta.url), 'utf8');
  let timerId = 0;
  runInNewContext(source, {
    document,
    window: { setTimeout(callback, delay) { const id = ++timerId; timers.push({ callback, delay, id }); return id; }, clearTimeout(id) { const index = timers.findIndex(t => t.id === id); if (index >= 0) timers.splice(index, 1); } },
    crypto: { randomUUID() { return '00000000-0000-4000-8000-000000000001'; } },
    TextEncoder, presentation, strategyFailure, sourceFailure, sourcePresentation, paperMoney, paperReason, paperReceipt, paperFailure, paperChecking, paperRecovery, learningExample, invoke,
  });
  starting = false;
  await setImmediate();
  return {
    elements, element, calls, pending, timers, document, invoke, flush: setImmediate,
    click: id => element(id).handlers.click(),
    edit(text) { element('strategy-json').value = text; element('strategy-json').handlers.input(); },
  };
}
