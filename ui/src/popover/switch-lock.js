// Shared lock for noise-control inputs (mode buttons and intensity segments).
// Locks on click, unlocks when the earbuds confirm the switch
// (device state noisePending -> false). A safety timeout makes sure the UI
// can never stay stuck if no confirmation ever arrives.
//
// Loaded as a classic script before renderer.js; exposes one global factory.

window.createSwitchLock = function createSwitchLock({ onToggle, safetyMs = 6000 }) {
  let locked = false;
  let timer = null;

  function set(value) {
    if (value === locked) return;
    locked = value;
    clearTimeout(timer);
    if (locked) timer = setTimeout(() => set(false), safetyMs);
    onToggle(locked);
  }

  return {
    get locked() {
      return locked;
    },
    lock: () => set(true),
    sync: (pending) => set(Boolean(pending)),
  };
};
