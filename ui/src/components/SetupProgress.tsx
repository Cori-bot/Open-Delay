import './SetupProgress.css';

// Adapted from Sudharshan Pani's Flowpane Stepper (MIT); see docs/21ST-ATTRIBUTION.md.
export function SetupProgress({ steps, current, onBack, label, disabled = false }: {
  steps: string[];
  current: number;
  onBack: (index: number) => void;
  label: string;
  disabled?: boolean;
}) {
  return <ol className="setup-progress" aria-label={label}>
    {steps.map((step, index) => {
      const complete = index < current;
      const content = <>
        <span className="setup-progress-marker" aria-hidden="true">{complete ? '✓' : index + 1}</span>
        <span className="setup-progress-label">{step}</span>
      </>;
      return <li key={index} data-state={complete ? 'complete' : index === current ? 'current' : 'upcoming'} aria-current={index === current ? 'step' : undefined}>
        {complete ? <button type="button" disabled={disabled} onClick={() => onBack(index)}>{content}</button> : <div>{content}</div>}
      </li>;
    })}
  </ol>;
}
