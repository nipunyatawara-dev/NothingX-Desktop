// Controls page (gestures), styled after macOS AirPods settings: pick the left
// or right bud, then one grouped list of its gestures. Each value is a pop-up
// button that opens a native macOS menu. When Tap and Hold is set to Noise
// Control, checkboxes choose which modes it switches between.
//
// Opened by clicking the left or right bud in the main panel's battery row
// (on models with gesture support); Back or Esc returns.
// Loaded as a classic script before renderer.js; exposes one global factory.

const NOISE_VALUE = 'noise';
const NOISE_MODES = [
  ['anc', 'Noise Cancellation'],
  ['transparency', 'Transparency'],
  ['off', 'Off'],
];

window.createControls = function createControls({ api, views, symbols }) {
  const $ = (id) => document.getElementById(id);
  // Battery row buds double as the way in: click one to edit its gestures.
  const batteryBuds = {
    left: document.querySelector('.bud[data-part="left"]'),
    right: document.querySelector('.bud[data-part="right"]'),
  };
  const sideTitle = $('controls-side-title');
  const gestureGroup = $('controls-gestures');
  const noiseSection = $('controls-noise');
  const noiseGroup = $('controls-noise-modes');
  const budCards = [...document.querySelectorAll('.bud-card')];

  let state = null;
  let side = 'left';

  for (const [budSide, el] of Object.entries(batteryBuds)) {
    el.addEventListener('click', () => {
      if (!state?.connected || !state.capabilities.gestures) return;
      side = budSide;
      render(state);
      views.open('view-controls');
    });
  }
  $('controls-back').addEventListener('click', () => views.back());

  for (const card of budCards) {
    card.addEventListener('click', () => {
      side = card.dataset.side;
      render(state);
    });
  }

  // Noise cycle code -> { anc, transparency, off }, from the device capabilities.
  const noiseCycles = () => state?.capabilities.gestures?.noiseCycles ?? {};
  // Current UI value for a slot: 'noise' for any noise cycle code.
  const uiValue = (code) => (code in noiseCycles() ? NOISE_VALUE : code);

  async function openMenu(slot, button) {
    const current = uiValue(state.gestures[side][slot.key]);
    const rect = button.getBoundingClientRect();
    const chosen = await api.popupMenu(
      slot.actions.map((a) => ({ label: a.label, value: a.value, checked: a.value === current })),
      rect.left,
      rect.bottom + 4
    );
    if (chosen !== null && chosen !== undefined && chosen !== current) {
      api.setGesture(side, slot.key, chosen);
    }
  }

  function gestureRow(slot) {
    const code = state.gestures[side][slot.key];
    const value = uiValue(code);
    const label = slot.actions.find((a) => a.value === value)?.label ?? '—';
    const pending = Boolean(state.gestures.pending[`${side}.${slot.key}`]);

    const row = document.createElement('div');
    row.className = 'group-row';

    const name = document.createElement('span');
    name.className = 'group-label';
    name.textContent = slot.label;

    const button = document.createElement('button');
    button.className = 'popup-button';
    button.classList.toggle('pending', pending);
    button.setAttribute('aria-haspopup', 'menu');
    button.setAttribute('aria-label', `${slot.label}: ${label}`);
    const text = document.createElement('span');
    text.textContent = label;
    const spinner = document.createElement('span');
    spinner.className = 'spinner';
    spinner.setAttribute('aria-hidden', 'true');
    const chevrons = document.createElement('i');
    chevrons.className = 'sym';
    if (symbols.popUpChevrons) chevrons.style.setProperty('--sym', `url("${symbols.popUpChevrons}")`);
    button.append(text, spinner, chevrons);
    button.addEventListener('click', () => openMenu(slot, button));

    row.append(name, button);
    return row;
  }

  function noiseRows(slotKey, code) {
    const cycle = noiseCycles()[code];
    const onCount = Object.values(cycle).filter(Boolean).length;
    return NOISE_MODES.map(([mode, label]) => {
      const checked = cycle[mode];
      const row = document.createElement('label');
      row.className = 'group-row check-row';
      const box = document.createElement('input');
      box.type = 'checkbox';
      box.className = 'checkbox';
      box.checked = checked;
      // Keep at least two modes: the last two can't be unticked.
      box.disabled = checked && onCount <= 2;
      box.addEventListener('change', () => {
        api.setGestureNoiseCycle(side, slotKey, { ...cycle, [mode]: box.checked });
      });
      const text = document.createElement('span');
      text.textContent = label;
      row.append(box, text);
      return row;
    });
  }

  function render(next) {
    state = next;
    const caps = next.capabilities.gestures;
    const enabled = next.connected && Boolean(caps);
    for (const [budSide, el] of Object.entries(batteryBuds)) {
      el.classList.toggle('clickable', enabled);
      el.title = enabled ? `${budSide === 'left' ? 'Left' : 'Right'} earbud controls` : '';
    }
    if (!enabled) {
      if (views.isOpen('view-controls')) views.back();
      return;
    }

    for (const card of budCards) {
      const selected = card.dataset.side === side;
      card.classList.toggle('on', selected);
      card.setAttribute('aria-checked', String(selected));
    }
    sideTitle.textContent = side === 'left' ? 'Left Earbud' : 'Right Earbud';
    gestureGroup.replaceChildren(...caps.slots.map(gestureRow));

    // Noise Control checkboxes for the gesture that's set to Noise Control.
    const noiseSlot = caps.slots.find((s) => s.actions.some((a) => a.value === NOISE_VALUE));
    const noiseCode = noiseSlot ? next.gestures[side][noiseSlot.key] : undefined;
    noiseSection.hidden = !(noiseCode in noiseCycles());
    if (!noiseSection.hidden) noiseGroup.replaceChildren(...noiseRows(noiseSlot.key, noiseCode));
  }

  return { render };
};
