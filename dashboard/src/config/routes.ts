export const routes = {
  login: '/login',
  dashboard: '/dashboard',
  crawls: '/crawls',
  results: (id: string) => `/results/${id}`,
  users: '/users',
  settings: '/settings',
} as const;
