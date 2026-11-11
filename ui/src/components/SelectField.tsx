import { Select } from '@base-ui/react/select';
import { Check, ChevronDown } from 'lucide-react';
import './SelectField.css';

type SelectOption = { value: string; label: string };

/** Accessible single-value select, shared by settings, onboarding and the overlay editor. */
export function SelectField({ label, value, options, onChange, disabled = false }: {
  label: string;
  value: string | number;
  options: SelectOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <Select.Root value={String(value)} items={options} disabled={disabled} onValueChange={(next) => { if (next !== null) onChange(next); }}>
      <Select.Trigger className="select-trigger" aria-label={label}>
        <Select.Value />
        <Select.Icon className="select-icon"><ChevronDown size={16} strokeWidth={1.8} aria-hidden="true" /></Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        <Select.Positioner className="select-positioner" sideOffset={6} alignItemWithTrigger={false}>
          <Select.Popup className="select-popup">
            <Select.List className="select-list">
              {options.map((option) => <Select.Item className="select-item" key={option.value} value={option.value}>
                <Select.ItemText>{option.label}</Select.ItemText>
                <Select.ItemIndicator className="select-indicator"><Check size={15} strokeWidth={2} aria-hidden="true" /></Select.ItemIndicator>
              </Select.Item>)}
            </Select.List>
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  );
}
