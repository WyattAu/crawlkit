import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { apiClient } from '../services/api_client';

interface User {
  id: string;
  email: string;
  name: string;
  roles: string[];
}

interface AuthState {
  token: string | null;
  user: User | null;
  isAuthenticated: boolean;
  login: (email: string, password: string) => Promise<boolean>;
  logout: () => void;
  setToken: (token: string) => void;
  refreshAuth: () => Promise<boolean>;
}

export const useAuth = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,
      user: null,
      isAuthenticated: false,
      login: async (email: string, password: string) => {
        try {
          const data = await apiClient.login(email, password);
          apiClient.setToken(data.token);
          set({ token: data.token, isAuthenticated: true });
          return true;
        } catch {
          return false;
        }
      },
      logout: () => {
        set({ token: null, user: null, isAuthenticated: false });
      },
      setToken: (token: string) => {
        apiClient.setToken(token);
        set({ token, isAuthenticated: true });
      },
      refreshAuth: async () => {
        const { token } = get();
        if (!token) return false;
        try {
          apiClient.setToken(token);
          const data = await apiClient.refreshToken();
          apiClient.setToken(data.token);
          set({ token: data.token, isAuthenticated: true });
          return true;
        } catch {
          set({ token: null, user: null, isAuthenticated: false });
          return false;
        }
      },
    }),
    {
      name: 'auth-storage',
    }
  )
);
