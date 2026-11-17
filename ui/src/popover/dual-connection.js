// Dual Connection on the Quick Settings page: a tile turns it on or off, and
// while it's on the earbuds' remembered devices are listed underneath as
// Bluetooth-menu style rows. Click a row to connect or disconnect that device;
// at most 2 can be connected, and this Mac can't be disconnected from here.
//
// Loaded as a classic script before renderer.js; exposes one global factory.

// The earbuds only report names, so guess an icon from the name.
const DEVICE_ICON_RULES = [
  [/iphone|phone|pixel|galaxy|oneplus|nothing phone|redmi|xiaomi|android/i, 'dev_phone'],
  [/ipad|tablet|\btab\b/i, 'dev_tablet'],
  [/watch/i, 'dev_watch'],
  [/\btv\b|television|bravia/i, 'dev_tv'],
  [/imac|desktop|\bpc\b|studio/i, 'dev_desktop'],
  [/mac|book|laptop|thinkpad|surface|notebook/i, 'dev_laptop'],
];

function deviceIconKey(device) {
  if (device.isSelf) return 'dev_laptop';
  return DEVICE_ICON_RULES.find(([re]) => re.test(device.name))?.[1] ?? 'dev_other';
}

window.createDualConnectionTile = function createDualConnectionTile({ api, symbols }) {
  const $ = (id) => document.getElementById(id);
  const tile = $('qs-dual');
  const panel = $('qs-dual-panel');
  const list = $('qs-dual-list');
  const count = $('qs-dual-count');
  const note = $('qs-dual-note');

  let state = null;

  tile.addEventListener('click', () => {
    const dual = state?.dual;
    if (!dual || dual.pending || dual.enabled === null) return;
    api.setDualConnection(!dual.enabled);
  });

  list.addEventListener('click', (e) => {
    const row = e.target.closest('.dual-row');
    if (!row || row.classList.contains('locked') || !state) return;
    const device = state.dual.devices.find((d) => d.mac === row.dataset.mac);
    if (device) api.setDualDevice(device.mac, !device.isConnected);
  });

  function statusText(device, busy) {
    if (busy) return device.isConnected ? 'Disconnecting…' : 'Connecting…';
    if (device.isSelf) return 'This Mac · Connected';
    return device.isConnected ? 'Connected' : 'Not connected';
  }

  function renderRows(dual) {
    const connected = dual.devices.filter((d) => d.isConnected).length;
    const full = connected >= dual.maxConnected;

    list.replaceChildren(
      ...dual.devices.map((device) => {
        const busy = dual.busyMac === device.mac;
        // Locked: this Mac, another device mid-change, or no free slot to connect.
        const locked = device.isSelf || (dual.busyMac && !busy) || busy || (!device.isConnected && full);

        const row = document.createElement('button');
        row.className = 'dual-row';
        row.dataset.mac = device.mac;
        row.classList.toggle('on', device.isConnected);
        row.classList.toggle('locked', Boolean(locked));
        row.classList.toggle('dim', !device.isConnected && !busy && (full || Boolean(dual.busyMac)));
        row.setAttribute('aria-pressed', String(device.isConnected));

        const badge = document.createElement('span');
        badge.className = 'qs-icon';
        badge.classList.toggle('pending', busy);
        const icon = document.createElement('i');
        icon.className = 'sym';
        const url = symbols[deviceIconKey(device)];
        if (url) icon.style.setProperty('--sym', `url("${url}")`);
        const spinner = document.createElement('span');
        spinner.className = 'spinner';
        spinner.setAttribute('aria-hidden', 'true');
        badge.append(icon, spinner);

        const text = document.createElement('span');
        text.className = 'qs-text';
        const name = document.createElement('span');
        name.className = 'qs-name';
        name.textContent = device.isSelf ? (device.name || 'This Mac') : (device.name || 'Unknown device');
        const sub = document.createElement('span');
        sub.className = 'qs-state';
        sub.textContent = statusText(device, busy);
        text.append(name, sub);

        row.append(badge, text);
        return row;
      })
    );

    count.textContent = `${connected} of ${dual.maxConnected} connected`;
    const message = dual.devices.length === 0
      ? 'Loading remembered devices…'
      : full ? `${dual.maxConnected} devices connected. Disconnect one to connect another.` : '';
    note.hidden = !message;
    note.textContent = message;
  }

  function render(next) {
    state = next;
    const caps = next.capabilities;
    const dual = next.dual;
    tile.hidden = !next.connected || !caps.dualConnection;
    if (tile.hidden) {
      panel.hidden = true;
      return;
    }

    const on = dual.enabled === true;
    const connected = dual.devices.filter((d) => d.isConnected).length;
    tile.classList.toggle('on', on);
    tile.setAttribute('aria-checked', String(on));
    tile.querySelector('.qs-icon').classList.toggle('pending', dual.pending);
    tile.querySelector('.qs-state').textContent = dual.enabled === null
      ? '—'
      : dual.pending && caps.dualRestarts ? 'Restarting buds…'
        : on ? `${connected} of ${dual.maxConnected} connected` : 'Off';

    panel.hidden = !on;
    if (on) renderRows(dual);
  }

  return { render };
};
