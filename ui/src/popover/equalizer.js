// Equalizer page: preset tiles with SF Symbols (same style as Quick Settings)
// and, for Custom, a live curve over Bass / Mid / Treble sliders (−6..+6 dB).
// Opened from the equalizer button in the main header.
//
// Loaded as a classic script before renderer.js; exposes one global factory.

const EQ_GAIN_LIMIT = 6;
const CURVE = { width: 260, height: 64, pad: 6 };
const BANDS = ['bass', 'mid', 'treble'];

const formatGain = (v) => (v > 0 ? `+${v} dB` : v < 0 ? `−${Math.abs(v)} dB` : '0 dB');

// Smooth curve: flat-ish bass shelf on the left, mid peak in the centre,
// treble shelf on the right.
function curvePath({ bass, mid, treble }) {
  const { width: w, height: h, pad } = CURVE;
  const y = (gain) => h / 2 - (gain / EQ_GAIN_LIMIT) * (h / 2 - pad);
  const [yb, ym, yt] = [y(bass), y(mid), y(treble)];
  return `M0 ${yb} C${w * 0.22} ${yb} ${w * 0.32} ${ym} ${w * 0.5} ${ym} `
    + `S${w * 0.78} ${yt} ${w} ${yt}`;
}

window.createEqualizer = function createEqualizer({ api, views, symbols }) {
  const $ = (id) => document.getElementById(id);
  const button = $('equalizer');
  const tiles = $('eq-tiles');
  const custom = $('eq-custom');
  const line = $('eq-line');
  const area = $('eq-area');
  const sliders = Object.fromEntries(BANDS.map((b) => [b, custom.querySelector(`input[data-band="${b}"]`)]));
  const values = Object.fromEntries(BANDS.map((b) => [b, custom.querySelector(`[data-value-for="${b}"]`)]));

  let state = null;
  let builtKey = '';

  button.addEventListener('click', () => {
    if (state?.connected && state.capabilities.eq.presets.length) views.open('view-eq');
  });
  $('eq-back').addEventListener('click', () => views.back());

  // ---- preset tiles ---------------------------------------------------------

  function buildTiles(presets) {
    const key = presets.map((p) => p.id).join();
    if (key === builtKey) return;
    builtKey = key;
    tiles.replaceChildren(
      ...presets.map((preset) => {
        const tile = document.createElement('button');
        tile.className = 'qs-tile eq-tile';
        tile.dataset.id = String(preset.id);
        tile.setAttribute('role', 'radio');

        const badge = document.createElement('span');
        badge.className = 'qs-icon';
        const icon = document.createElement('i');
        icon.className = 'sym';
        const url = symbols[`eq_${preset.key}`];
        if (url) icon.style.setProperty('--sym', `url("${url}")`);
        const spinner = document.createElement('span');
        spinner.className = 'spinner';
        spinner.setAttribute('aria-hidden', 'true');
        badge.append(icon, spinner);

        const name = document.createElement('span');
        name.className = 'qs-name';
        name.textContent = preset.label;

        tile.append(badge, name);
        return tile;
      })
    );
  }

  tiles.addEventListener('click', (e) => {
    const tile = e.target.closest('.eq-tile');
    if (!tile || !state || state.eq.pending) return;
    const id = Number(tile.dataset.id);
    if (id !== state.eq.preset) api.setEqPreset(id);
  });

  // ---- custom EQ ------------------------------------------------------------

  function paintSlider(input, gain) {
    // Fill from the centre (0 dB) towards the value.
    const pct = ((gain + EQ_GAIN_LIMIT) / (EQ_GAIN_LIMIT * 2)) * 100;
    const [from, to] = gain >= 0 ? [50, pct] : [pct, 50];
    input.style.setProperty('--from', `${from}%`);
    input.style.setProperty('--to', `${to}%`);
  }

  function paintCustom(bands) {
    for (const band of BANDS) {
      if (document.activeElement !== sliders[band]) sliders[band].value = String(bands[band]);
      values[band].textContent = formatGain(bands[band]);
      paintSlider(sliders[band], bands[band]);
    }
    const d = curvePath(bands);
    line.setAttribute('d', d);
    area.setAttribute('d', `${d} L${CURVE.width} ${CURVE.height / 2} L0 ${CURVE.height / 2} Z`);
  }

  for (const band of BANDS) {
    sliders[band].addEventListener('input', () => {
      const gain = Number(sliders[band].value);
      const current = state?.eq.custom ?? { bass: 0, mid: 0, treble: 0 };
      paintCustom({ ...current, [band]: gain }); // instant feedback while dragging
      api.setCustomEq({ [band]: gain });
    });
  }

  // ---- render ---------------------------------------------------------------

  function render(next) {
    state = next;
    const eqCaps = next.capabilities.eq;
    button.disabled = !next.connected || eqCaps.presets.length === 0;
    if (!next.connected) {
      if (views.isOpen('view-eq')) views.back();
      return;
    }

    buildTiles(eqCaps.presets);
    for (const tile of tiles.children) {
      const selected = Number(tile.dataset.id) === next.eq.preset;
      tile.classList.toggle('on', selected);
      tile.setAttribute('aria-checked', String(selected));
      tile.querySelector('.qs-icon').classList.toggle('pending', selected && next.eq.pending);
    }
    tiles.classList.toggle('busy', next.eq.pending);

    const showCustom = eqCaps.customId !== null && next.eq.preset === eqCaps.customId;
    custom.hidden = !showCustom;
    if (showCustom) paintCustom(next.eq.custom ?? { bass: 0, mid: 0, treble: 0 });
  }

  return { render };
};
