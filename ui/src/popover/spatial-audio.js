// Spatial Audio tile on the Quick Settings page.
// Models with just Off and one mode (e.g. CMF Buds 2 Plus: Off / Fixed): the
// tile switches between them. Models with more modes: the tile opens a short
// list underneath to pick from. Hidden on models without spatial audio.
//
// Loaded as a classic script before renderer.js; exposes one global factory.

window.createSpatialAudioTile = function createSpatialAudioTile({ api, views }) {
  const tile = document.getElementById('qs-spatial');
  const list = document.getElementById('qs-spatial-list');

  let state = null;
  let listOpen = false;
  let builtKey = '';

  const modes = () => state?.capabilities.spatialModes ?? [];
  const pending = () => Boolean(state?.quick.pending.spatial);

  function choose(mode) {
    if (!state || pending() || mode === state.quick.spatial) return;
    api.setSpatialAudio(mode);
  }

  tile.addEventListener('click', () => {
    if (!state || pending()) return;
    const available = modes();
    if (available.length === 2) {
      // Off <-> the one other mode
      choose(state.quick.spatial === 'off' ? available[1].key : 'off');
    } else {
      listOpen = !listOpen;
      render(state);
    }
  });

  list.addEventListener('click', (e) => {
    const row = e.target.closest('.qs-list-row');
    if (row) choose(row.dataset.mode);
  });

  views.onBack(() => {
    listOpen = false;
  });

  function buildList(available) {
    const key = available.map((m) => m.key).join();
    if (key === builtKey) return;
    builtKey = key;
    list.replaceChildren(
      ...available.map(({ key: mode, label }) => {
        const row = document.createElement('button');
        row.className = 'qs-list-row';
        row.dataset.mode = mode;
        row.setAttribute('role', 'radio');
        const check = document.createElement('span');
        check.className = 'qs-check';
        check.textContent = '✓';
        check.setAttribute('aria-hidden', 'true');
        const text = document.createElement('span');
        text.textContent = label;
        row.append(check, text);
        return row;
      })
    );
  }

  function render(next) {
    state = next;
    const available = modes();
    tile.hidden = !next.connected || available.length === 0;
    if (tile.hidden) {
      list.hidden = true;
      return;
    }

    const current = next.quick.spatial; // null until read
    const label = available.find((m) => m.key === current)?.label ?? '—';
    tile.classList.toggle('on', Boolean(current) && current !== 'off');
    tile.setAttribute('aria-expanded', String(listOpen));
    tile.querySelector('.qs-icon').classList.toggle('pending', pending());
    tile.querySelector('.qs-state').textContent = label;

    const hasList = available.length > 2;
    list.hidden = !(hasList && listOpen);
    if (!hasList) return;
    buildList(available);
    list.classList.toggle('busy', pending());
    for (const row of list.children) {
      const selected = row.dataset.mode === current;
      row.classList.toggle('on', selected);
      row.setAttribute('aria-checked', String(selected));
    }
  }

  return { render };
};
