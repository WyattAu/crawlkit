# Web Dashboard Testing Guide

## Overview

This guide covers testing the crawlkit React web dashboard.

## Test Types

### Unit Tests

**Coverage Target:** 80%

**Focus Areas:**
- Components (rendering, interaction)
- Hooks (state management, side effects)
- Services (API calls, data transformation)

**Example:**
```typescript
test('MetricCard renders correctly', () => {
  render(<MetricCard title="Crawls" value={10} icon="spider" />);
  expect(screen.getByText('Crawls')).toBeInTheDocument();
  expect(screen.getByText('10')).toBeInTheDocument();
});
```

### Integration Tests

**Coverage Target:** 70%

**Focus Areas:**
- Page rendering
- API integration
- Navigation flows

**Example:**
```typescript
test('Dashboard loads metrics', async () => {
  render(<DashboardPage />);
  await waitFor(() => {
    expect(screen.getByText('Total Crawls')).toBeInTheDocument();
  });
});
```

### E2E Tests

**Coverage Target:** 50%

**Focus Areas:**
- User flows (login, crawl, results)
- Cross-browser (Chrome, Firefox, Safari)
- Responsive (mobile, tablet, desktop)

**Example:**
```typescript
test('User can login and view dashboard', async ({ page }) => {
  await page.goto('http://localhost:3000/login');
  await page.fill('input[name="email"]', 'user@example.com');
  await page.fill('input[name="password"]', 'password');
  await page.click('button[type="submit"]');
  await expect(page).toHaveURL('/dashboard');
});
```

## Test Environment

### Browsers

- **Chrome:** Latest stable
- **Firefox:** Latest stable
- **Safari:** Latest stable
- **Edge:** Latest stable

### Viewports

- **Mobile:** 375x667 (iPhone SE)
- **Tablet:** 768x1024 (iPad)
- **Desktop:** 1920x1080 (Full HD)

## Test Automation

### CI/CD Integration

```yaml
# GitHub Actions
dashboard-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: '20'
    - run: cd dashboard && npm ci
    - run: cd dashboard && npm run test
    - run: cd dashboard && npm run build
    - run: cd dashboard && npx playwright test
```

### Testing Tools

- **Vitest**: Unit and integration tests
- **Playwright**: E2E tests
- **Testing Library**: Component testing
- **Storybook**: Visual testing

## Test Data

### Mock Data

```typescript
const mockCrawl = {
  crawl_id: 'test-123',
  status: 'completed',
  target_url: 'https://example.com',
  pages_crawled: 50,
  total_issues: 12,
};
```

### Test Accounts

- **Admin:** admin@crawlkit.local / admin123
- **Editor:** editor@crawlkit.local / editor123
- **Viewer:** viewer@crawlkit.local / viewer123

## Performance Testing

### Lighthouse Scores

- **Performance:** >90
- **Accessibility:** >90
- **Best Practices:** >90
- **SEO:** >90

### Core Web Vitals

- **LCP:** <2.5s
- **FID:** <100ms
- **CLS:** <0.1

### Bundle Size

- **Initial:** <200KB
- **Lazy loaded:** <50KB per chunk

## Accessibility Testing

### WCAG 2.1 AA

- [ ] Keyboard navigation
- [ ] Screen reader support
- [ ] Color contrast
- [ ] Focus indicators
- [ ] ARIA labels

### Tools

- **axe-core**: Automated accessibility testing
- **Lighthouse**: Accessibility audit
- **WAVE**: Web accessibility evaluation

## Security Testing

### Authentication

- [ ] JWT token stored securely
- [ ] Token refresh works
- [ ] Logout clears tokens
- [ ] Session timeout works

### Data Protection

- [ ] HTTPS enforced
- [ ] XSS prevention
- [ ] CSRF protection
- [ ] Input validation

## Visual Regression Testing

### Screenshot Comparison

```typescript
test('Dashboard matches snapshot', async ({ page }) => {
  await page.goto('http://localhost:3000/dashboard');
  await expect(page).toHaveScreenshot('dashboard.png');
});
```

### Component Snapshots

```typescript
test('MetricCard matches snapshot', () => {
  const { container } = render(<MetricCard title="Crawls" value={10} icon="spider" />);
  expect(container).toMatchSnapshot();
});
```

## Test Reporting

### Test Report Template

```markdown
# Web Dashboard Test Report

## Environment
- Date: YYYY-MM-DD
- Browser: Chrome/Firefox/Safari
- Version: X.X.X

## Results

### Unit Tests
- Passed: X
- Failed: X
- Coverage: X%

### Integration Tests
- Passed: X
- Failed: X
- Coverage: X%

### E2E Tests
- Passed: X
- Failed: X
- Duration: X minutes

### Performance
- Lighthouse score: X
- LCP: Xms
- FID: Xms
- CLS: X

### Accessibility
- WCAG 2.1 AA: PASS/FAIL
- Screen reader: PASS/FAIL

### Visual Regression
- Snapshots matched: X
- Snapshots updated: X

### Issues
- [List any issues encountered]
```

## Continuous Testing

### Automated Testing

- Run unit tests on every commit
- Run integration tests on every PR
- Run E2E tests nightly
- Run visual regression weekly

### Manual Testing

- Test on real browsers monthly
- Test new features before release
- Test accessibility quarterly
- Test performance monthly
