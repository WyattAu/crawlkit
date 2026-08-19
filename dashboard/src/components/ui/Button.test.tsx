/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import Button from './Button';

describe('Button', () => {
  it('renders children and handles clicks', async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Start crawl</Button>);
    const button = screen.getByRole('button', { name: /start crawl/i });
    expect(button).toBeDefined();
    await userEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('disables interaction when disabled', async () => {
    const onClick = vi.fn();
    render(
      <Button onClick={onClick} disabled>
        Start crawl
      </Button>
    );
    const button = screen.getByRole('button', { name: /start crawl/i });
    expect(button.hasAttribute('disabled')).toBe(true);
    await userEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it('applies variant classes', () => {
    const { container } = render(<Button variant="danger">Delete</Button>);
    expect(container.querySelector('button')?.className).toContain('bg-red-600');
  });
});
