import { apiClient } from './api_client';

export interface LoginResponse {
  token: string;
  user?: {
    id: string;
    email: string;
    name: string;
  };
}

export const authService = {
  async login(email: string, password: string): Promise<LoginResponse> {
    const response = await apiClient.login(email, password);
    return response as unknown as LoginResponse;
  },

  async validateToken(token: string): Promise<boolean> {
    try {
      apiClient.setToken(token);
      await apiClient.health();
      return true;
    } catch {
      return false;
    }
  },
};
