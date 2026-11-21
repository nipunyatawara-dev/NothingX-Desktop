// Exercise battery/model rendering without hardware or a browser dependency.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const nodes = new Map();
function element() {
  const children = new Map();
  const classes = new Set();
  return {
    hidden: false, textContent: '', style: {},
    classList: {
      toggle(name, force) {
        if (force === undefined) {
          if (classes.has(name)) classes.delete(name);
          else classes.add(name);
        } else if (force) {
          classes.add(name);
        } else {
          classes.delete(name);
        }
      },
      contains(name) { return classes.has(name); },
    },
    querySelector(selector) {
      if (!children.has(selector)) children.set(selector, element());
      return children.get(selector);
    },
  };
}
const document = {
  querySelector(selector) {
    if (!nodes.has(selector)) nodes.set(selector, element());
    return nodes.get(selector);
  },
};
const context = { document, window: {
  createSwitchLock: () => ({}),
  nx: { init: () => new Promise(() => {}) },
} };
vm.createContext(context);
vm.runInContext(fs.readFileSync(path.join(__dirname, '../ui/src/popover/renderer.js'), 'utf8'), context);
const bud = part => document.querySelector(`.bud[data-part="${part}"]`);
context.renderBattery({ stereo: { level: 72, charging: false }, left: null, right: null, case: null });
assert.equal(bud('stereo').hidden, false);
assert.equal(bud('stereo').querySelector('.pct').textContent, '72%');
assert.equal(bud('left').hidden, true);
assert.equal(bud('right').hidden, true);
context.renderBattery({ stereo: null, left: { level: 87 }, right: { level: 80 }, case: null });
assert.equal(bud('stereo').hidden, true);
assert.equal(bud('left').hidden, false);
assert.equal(bud('right').hidden, false);
assert.equal(bud('left').querySelector('.pct').textContent, '87%');
assert.equal(bud('case').hidden, true);
context.renderCapabilities({ noiseCancellation: false, strengths: [], adaptive: false, transparency: false });
assert.equal(document.querySelector('#noise-section').hidden, true);
context.renderCapabilities({ noiseCancellation: true, strengths: ['low', 'mid', 'high'], adaptive: true, transparency: true });
assert.equal(document.querySelector('#noise-section').hidden, false);
assert.equal(document.querySelector('.option[data-mode="anc"]').hidden, false);
context.renderCapabilities({ noiseCancellation: true, strengths: [], adaptive: false, transparency: true });
assert.equal(document.querySelector('.option[data-mode="anc"]').hidden, false);

// Fallback visibility and header states across idle, unrecognized, unresponsive, limited.
context.renderHeader({ connected: false, name: null, fallback: { mode: 'none', deviceName: null } });
context.renderFallback({ mode: 'none', deviceName: null });
assert.equal(document.querySelector('#fallback-card').hidden, true);
assert.equal(document.querySelector('#panel').classList.contains('fallback-full'), false);
assert.equal(document.querySelector('#device-name').textContent, 'No Earbuds');

context.renderHeader({ connected: false, name: null, fallback: { mode: 'unrecognized', deviceName: 'CMF Buds 3' } });
context.renderFallback({ mode: 'unrecognized', deviceName: 'CMF Buds 3' });
assert.equal(document.querySelector('#fallback-card').hidden, false);
assert.equal(document.querySelector('#panel').classList.contains('fallback-full'), true);
assert.equal(document.querySelector('#fallback-card').classList.contains('compact'), false);
assert.equal(document.querySelector('#device-name').textContent, 'CMF Buds 3');
assert.equal(document.querySelector('#device-status').textContent, 'Not recognized');

context.renderHeader({ connected: true, name: 'CMF Buds Pro 2', fallback: { mode: 'unresponsive', deviceName: 'CMF Buds Pro 2' } });
context.renderFallback({ mode: 'unresponsive', deviceName: 'CMF Buds Pro 2' });
assert.equal(document.querySelector('#fallback-card').hidden, false);
assert.equal(document.querySelector('#panel').classList.contains('fallback-full'), true);
assert.equal(document.querySelector('#device-status').textContent, 'No response');

context.renderHeader({ connected: true, name: 'CMF Buds 3 Pro', fallback: { mode: 'limited', deviceName: 'CMF Buds 3 Pro' } });
context.renderFallback({ mode: 'limited', deviceName: 'CMF Buds 3 Pro' });
assert.equal(document.querySelector('#fallback-card').hidden, false);
assert.equal(document.querySelector('#panel').classList.contains('fallback-full'), false);
assert.equal(document.querySelector('#fallback-card').classList.contains('compact'), true);
assert.equal(document.querySelector('#device-status').textContent, 'Connected · Basic');

console.log('Battery, model-dependent controls, and fallback modes passed.');
