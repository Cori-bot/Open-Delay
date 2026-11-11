import { Toaster, toast } from 'sonner';

export { toast as notify };

export function Notifications() {
  return (
    <Toaster
      theme="dark"
      position="bottom-right"
      hotkey={[]}
      duration={3200}
      visibleToasts={3}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast: 'notification',
          title: 'notification-title',
          description: 'notification-description',
        },
      }}
    />
  );
}
