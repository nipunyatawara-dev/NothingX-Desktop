// Bass Enhance, Control Center style: a round speaker button turns it on/off,
// and a stepped capsule slider sets the level. Each step shows a waveform
// icon that grows with the level; filled steps turn white.
//
// Has its own lock so a bass change never blocks noise control and vice versa.
// Loaded as a classic script after switch-lock.js.

const WAVE_MIN_PX = 7;
const WAVE_MAX_PX = 15;

window.createBassControl = function createBassControl({ onChange, symbols }) {
  const section = document.getElementById('bass-section');
  const separator = document.getElementById('bass-separator');
  const toggle = document.getElementById('bass-toggle');
  const track = document.getElementById('bass-track');
  const fill = document.getElementById('bass-fill');
  const slots = document.getElementById('bass-slots');

  const lock = window.createSwitchLock({
    onToggle: (locked) => section.classList.toggle('busy', locked),
  });

  let levels = [];
  let current = { enabled: false, level: 1 };

  function buildSlots(list) {
    if (list.join() === levels.join()) return;
    levels = list;
    const steps = Math.max(list.length - 1, 1);
    slots.replaceChildren(
      ...list.map((_, i) => {
        const slot = document.createElement('span');
        slot.className = 'bass-slot';
        const icon = document.createElement('i');
        icon.className = 'sym';
        const size = WAVE_MIN_PX + ((WAVE_MAX_PX - WAVE_MIN_PX) * i) / steps;
        icon.style.width = icon.style.height = `${size}px`;
        if (symbols.bassWave) icon.style.setProperty('--sym', `url("${symbols.bassWave}")`);
        slot.append(icon);
        return slot;
      })
    );
    track.setAttribute('aria-valuemin', String(list[0] ?? 1));
    track.setAttribute('aria-valuemax', String(list[list.length - 1] ?? 1));
  }

  function paint(enabled, level, pending) {
    const index = levels.indexOf(level);
    const filled = enabled && index !== -1 ? index + 1 : 0;

    section.classList.toggle('bass-off', !enabled);
    toggle.setAttribute('aria-checked', String(enabled));
    toggle.classList.toggle('pending', pending);
    toggle.setAttribute('aria-busy', String(pending));

    fill.style.width = `${(filled / Math.max(levels.length, 1)) * 100}%`;
    [...slots.children].forEach((slot, i) => slot.classList.toggle('in', i < filled));
    track.setAttribute('aria-valuenow', String(level));
    track.setAttribute('aria-valuetext', enabled ? `Level ${level}` : 'Off');
  }

  // One change at a time: lock, show the target with a spinner, send.
  function request(enabled, level) {
    if (lock.locked) return;
    if (enabled === current.enabled && (!enabled || level === current.level)) return;
    lock.lock();
    paint(enabled, level, true);
    onChange(enabled ? { enabled, level } : { enabled });
  }

  toggle.addEventListener('click', () => request(!current.enabled, current.level));

  track.addEventListener('click', (e) => {
    if (!levels.length) return;
    const rect = track.getBoundingClientRect();
    const step = Math.ceil(((e.clientX - rect.left) / rect.width) * levels.length);
    const index = Math.min(Math.max(step, 1), levels.length) - 1;
    request(true, levels[index]);
  });

  track.addEventListener('keydown', (e) => {
    const delta = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 }[e.key];
    if (!delta || !levels.length) return;
    e.preventDefault();
    const index = Math.min(Math.max(levels.indexOf(current.level) + delta, 0), levels.length - 1);
    request(true, levels[index]);
  });

  function render(state) {
    const { bass, capabilities } = state;
    const visible = state.connected && capabilities.bassSteps > 0 && bass !== null;
    section.hidden = !visible;
    separator.hidden = !visible;
    if (!visible) return;

    lock.sync(state.bassPending);
    buildSlots(capabilities.bassLevels);
    current = { enabled: bass.enabled, level: bass.level };
    paint(bass.enabled, bass.level, state.bassPending);
  }

  return { render };
};
