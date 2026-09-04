import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { type ReactNode } from 'react';
import { AuthWrapper } from '../auth/auth-wrapper';
import { NotificationProvider } from '../notifications/notification-provider';
import { ThemeProvider } from '../theme/theme-provider';
import { Toaster } from '../ui/sonner';

const queryClient = new QueryClient();

export const Layout = ({ children }: { children: ReactNode }) => (
  <QueryClientProvider client={queryClient}>
    <ThemeProvider>
      <AuthWrapper>
        <NotificationProvider>
          <main>{children}</main>
        </NotificationProvider>
      </AuthWrapper>
      <Toaster />
    </ThemeProvider>
  </QueryClientProvider>
);
