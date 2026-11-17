// Reusable macOS-style segmented control.
// Items: { value, label, icon?, title? } — icon is an SF Symbol data URL.
// While a change is in flight the chosen segment shows a spinner and the
// shared lock blocks further clicks.
//
// Loaded as a classic script before the controls that use it.

window.createSegmentedControl = function createSegmentedControl({ container, lock, onSelect }) {
  let builtKey = '';

  function buildSegment({ value, label, icon, title }) {
    const btn = document.createElement('button');
    btn.className = icon ? 'segment' : 'segment no-icon';
    btn.dataset.value = value;
    btn.setAttribute('role', 'radio');
    btn.title = title ?? label;

    if (icon) {
      const glyph = document.createElement('i');
      glyph.className = 'sym';
      glyph.style.setProperty('--sym', `url("${icon}")`);
      btn.append(glyph);
    }

    const spinner = document.createElement('span');
    spinner.className = 'spinner';
    spinner.setAttribute('aria-hidden', 'true');

    const text = document.createElement('span');
    text.className = 'label';
    text.textContent = label;

    btn.append(spinner, text);
    return btn;
  }

  function setItems(items) {
    const key = items.map((i) => i.value).join();
    if (key === builtKey) return;
    builtKey = key;
    container.replaceChildren(...items.map(buildSegment));
  }

  function select(value, pending = false) {
    for (const btn of container.children) {
      const selected = btn.dataset.value === value;
      btn.setAttribute('aria-checked', String(selected));
      btn.classList.toggle('pending', selected && pending);
      btn.setAttribute('aria-busy', String(selected && pending));
    }
  }

  container.addEventListener('click', (e) => {
    const btn = e.target.closest('.segment');
    if (!btn || lock.locked || btn.getAttribute('aria-checked') === 'true') return;
    lock.lock(); // no more changes until the earbuds confirm this one
    select(btn.dataset.value, true); // instant feedback; device state confirms
    onSelect(btn.dataset.value);
  });

  return { setItems, select };
};
