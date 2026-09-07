export const routes = {
  login: '/login',
  dashboard: '/dashboard',
  crawls: '/crawls',
  results: (id: string) => `/results/${id}`,
  insights: '/insights',
  users: '/users',
  settings: '/settings',
} as const;
