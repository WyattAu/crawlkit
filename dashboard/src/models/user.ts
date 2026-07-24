export interface User {
  id: string;
  email: string;
  name: string;
  tenant_id: string;
  roles: string[];
  created_at: string;
  last_login?: string;
}

export interface CreateUserRequest {
  email: string;
  name: string;
  password: string;
  tenant_id: string;
  roles: string[];
}
