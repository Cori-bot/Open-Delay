import { Dialog } from '@base-ui/react/dialog';
import type { ReactNode } from 'react';
import { AppIcon } from './AppIcon';

export function Modal({ title, children, close }: { title: string; children: ReactNode; close: () => void }) {
  return (
    <Dialog.Root open onOpenChange={(open) => { if (!open) close(); }}>
      <Dialog.Portal>
        <Dialog.Backdrop className="dialog-backdrop" />
        <Dialog.Popup className="dialog" aria-describedby={undefined}>
          <div className="dialog-head">
            <Dialog.Title>{title}</Dialog.Title>
            <Dialog.Close className="icon-button" aria-label="Fermer / Close">
              <AppIcon name="close" />
            </Dialog.Close>
          </div>
          {children}
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
