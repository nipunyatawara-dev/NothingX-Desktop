// Exercise battery/model rendering without hardware or a browser dependency.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const nodes = new Map();
function element() {
  const children = new Map();
  return {
    hidden: false, textContent: '', style: {},
    classList: { toggle() {} },
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
console.log('Battery and model-dependent controls passed: headphones, earbuds, missing case and unsupported ANC.');
