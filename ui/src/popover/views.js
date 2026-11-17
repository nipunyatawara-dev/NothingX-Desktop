// Page navigation inside the popover: the main view plus sub-pages
// (Quick Settings, Equalizer, Controls) that slide in from the right. Back or Esc
// returns; reopening the panel always starts on the main view.
//
// Loaded as a classic script before the pages that use it.

window.createViews = function createViews({ mainId }) {
  const main = document.getElementById(mainId);
  let current = main;
  const backListeners = [];

  function animate(view, direction) {
    view.classList.remove('enter-forward', 'enter-back');
    void view.offsetWidth; // restart the slide-in animation
    view.classList.add(direction === 'forward' ? 'enter-forward' : 'enter-back');
  }

  function go(view, direction, { animated = true } = {}) {
    if (!view || view === current) return;
    current.hidden = true;
    view.hidden = false;
    if (animated) animate(view, direction);
    current = view;
    if (view === main) backListeners.forEach((fn) => fn());
  }

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') go(main, 'back');
  });
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) go(main, 'back', { animated: false });
  });

  return {
    open: (id) => go(document.getElementById(id), 'forward'),
    back: () => go(main, 'back'),
    isOpen: (id) => current.id === id,
    onBack: (fn) => backListeners.push(fn),
  };
};
