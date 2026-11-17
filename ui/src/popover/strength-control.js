// Noise-cancellation intensity: Low / Mid / High / Auto segments with SF
// Symbols. Shown only while noise cancellation is on and the model supports it.
//
// Loaded as a classic script after segmented-control.js.

const STRENGTH_SEGMENTS = {
  low: { label: 'Low', symbol: 'ancLow' },
  mid: { label: 'Mid', symbol: 'ancMid' },
  high: { label: 'High', symbol: 'ancHigh' },
  adaptive: { label: 'Auto', symbol: 'ancAdaptive', title: 'Adaptive' },
};

window.createStrengthControl = function createStrengthControl({ onChange, symbols, lock }) {
  const root = document.getElementById('strength');
  const segmented = window.createSegmentedControl({
    container: document.getElementById('strength-segments'),
    lock,
    onSelect: onChange,
  });

  function render(state) {
    const caps = state.capabilities;
    const visible = state.connected && state.noiseMode === 'anc' && caps.strengths.length > 0;
    root.hidden = !visible;
    if (!visible) return;

    const strengths = caps.adaptive ? [...caps.strengths, 'adaptive'] : caps.strengths;
    segmented.setItems(
      strengths.map((value) => {
        const { label, symbol, title } = STRENGTH_SEGMENTS[value];
        return { value, label, title, icon: symbols[symbol] };
      })
    );
    segmented.select(state.ancStrength, state.noisePending);
  }

  return { render };
};
