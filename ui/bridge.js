// Compatibility bridge only. The original renderer, markup and styles are unchanged.
(() => {
  let sequence = 0;
  const replies = new Map();
  const listeners = { state: new Set(), accent: new Set() };
  function send(command, args = [], reply = false) {
    const id = ++sequence;
    if (!reply) { window.ipc.postMessage(JSON.stringify({ id, command, args })); return; }
    return new Promise(resolve => {
      replies.set(id, resolve);
      window.ipc.postMessage(JSON.stringify({ id, command, args }));
    });
  }
  window.__nxReply = (id, value) => { const resolve = replies.get(id); replies.delete(id); resolve?.(value); };
  window.__nxState = state => { for (const listener of listeners.state) listener(state); };
  window.__nxAccent = value => { for (const listener of listeners.accent) listener(value); };
  window.nx = {
    init: () => send('init', [], true),
    setHeight: height => send('setHeight', [height]),
    onAccent: listener => listeners.accent.add(listener),
    onDeviceState: listener => listeners.state.add(listener),
    quit: () => send('quit'),
    popupMenu: (items, x, y) => send('popupMenu', [items, x, y], true),
  };
  for (const command of ['setNoiseMode', 'setAncStrength', 'setBass', 'setQuickToggle', 'ring', 'setSpatialAudio', 'setDualConnection', 'setDualDevice', 'setGesture', 'setGestureNoiseCycle', 'setEqPreset', 'setCustomEq']) {
    window.nx[command] = (...args) => send(command, args);
  }
  window.addEventListener('error', event => send('uiError', [event.message]));
  window.addEventListener('unhandledrejection', event => send('uiError', [String(event.reason)]));
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  send('motionPreference', [reducedMotion.matches]);
  reducedMotion.addEventListener('change', event => send('motionPreference', [event.matches]));
})();
