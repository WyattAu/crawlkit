import { useEffect, useState } from 'react';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import Modal from '../components/ui/Modal';
import { Plus, Trash2, Loader } from 'lucide-react';
import type { UserResponse } from '../models/types';

export default function UsersPage() {
  const { token } = useAuth();
  const [users, setUsers] = useState<UserResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [formLoading, setFormLoading] = useState(false);
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [formError, setFormError] = useState('');

  useEffect(() => {
    if (token) {
      apiClient.setToken(token);
      loadUsers();
    }
  }, [token]);

  async function loadUsers() {
    try {
      setLoading(true);
      const data = await apiClient.listUsers();
      setUsers(data);
    } catch (error) {
      console.error('Failed to load users:', error);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setFormLoading(true);
    setFormError('');
    try {
      await apiClient.createUser({ email, name, password, tenant_id: 'default', roles: ['viewer'] });
      setShowForm(false);
      setEmail('');
      setName('');
      setPassword('');
      loadUsers();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create user';
      setFormError(message);
      console.error('Failed to create user:', err);
    } finally {
      setFormLoading(false);
    }
  }

  async function handleDelete(userId: string) {
    if (!confirm('Are you sure you want to delete this user?')) return;
    try {
      await apiClient.deleteUser(userId);
      loadUsers();
    } catch (error) {
      console.error('Failed to delete user:', error);
    }
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Users</h1>
        <Button onClick={() => setShowForm(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Add User
        </Button>
      </div>

      {loading ? (
        <div role="status" className="flex items-center justify-center py-12">
          <Loader className="w-8 h-8 text-blue-500 animate-spin" />
          <span className="sr-only">Loading...</span>
        </div>
      ) : (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
          <div className="overflow-x-auto">
          <table className="w-full" aria-label="Users list">
            <caption className="sr-only">Registered users with roles and management actions</caption>
            <thead>
              <tr className="border-b border-gray-200 dark:border-gray-700">
                <th scope="col" className="text-left px-4 py-3 text-sm font-medium text-gray-500 dark:text-gray-400">Name</th>
                <th scope="col" className="text-left px-4 py-3 text-sm font-medium text-gray-500 dark:text-gray-400">Email</th>
                <th scope="col" className="text-left px-4 py-3 text-sm font-medium text-gray-500 dark:text-gray-400">Roles</th>
                <th scope="col" className="text-right px-4 py-3 text-sm font-medium text-gray-500 dark:text-gray-400">Actions</th>
              </tr>
            </thead>
            <tbody>
              {users.map((user) => (
                <tr key={user.id} className="border-b border-gray-100 dark:border-gray-700/50 last:border-0">
                  <td className="px-4 py-3 text-sm text-gray-900 dark:text-white">{user.name}</td>
                  <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">{user.email}</td>
                  <td className="px-4 py-3">
                    <div className="flex gap-1">
                      {user.roles.map((role) => (
                        <span
                          key={role}
                          className="px-2 py-0.5 text-xs rounded-full bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
                        >
                          {role}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <button
                      aria-label={`Delete ${user.name}`}
                      onClick={() => handleDelete(user.id)}
                      className="p-1 text-gray-400 hover:text-red-500 transition-colors"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </div>
      )}

      <Modal open={showForm} onOpenChange={setShowForm} title="Add User">
        <form onSubmit={handleCreate} className="space-y-4">
          {formError && (
            <div role="alert" className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
              {formError}
            </div>
          )}
          <Input
            label="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
          <Input
            label="Email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
          <Input
            label="Password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          <div className="flex justify-end gap-3 pt-4">
            <Button type="submit" disabled={formLoading}>
              {formLoading ? 'Creating...' : 'Create User'}
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
