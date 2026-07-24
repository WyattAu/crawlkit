# Mobile App Testing Guide

## Overview

This guide covers testing the crawlkit Flutter mobile app.

## Test Types

### Unit Tests

**Coverage Target:** 80%

**Focus Areas:**
- Business logic (use cases, repositories)
- Data models (serialization, validation)
- Service layer (API client, auth service)

**Example:**
```dart
test('CrawlRequest serialization', () {
  final request = CrawlRequest(
    startUrl: 'https://example.com',
    maxPages: 50,
  );
  final json = request.toJson();
  expect(json['start_url'], 'https://example.com');
  expect(json['max_pages'], 50);
});
```

### Widget Tests

**Coverage Target:** 70%

**Focus Areas:**
- UI components (buttons, cards, inputs)
- Navigation (routes, redirects)
- Forms (validation, submission)

**Example:**
```dart
testWidgets('Login screen renders correctly', (tester) async {
  await tester.pumpWidget(MaterialApp(home: LoginScreen()));
  expect(find.text('Login'), findsOneWidget);
  expect(find.byType(TextFormField), findsNWidgets(2));
});
```

### Integration Tests

**Coverage Target:** 60%

**Focus Areas:**
- API integration
- Authentication flow
- Crawl management

**Example:**
```dart
testWidgets('Full login flow', (tester) async {
  await tester.pumpWidget(MaterialApp(home: LoginScreen()));
  await tester.enterText(find.byType(TextFormField).first, 'user@example.com');
  await tester.enterText(find.byType(TextFormField).last, 'password');
  await tester.tap(find.text('Login'));
  await tester.pumpAndSettle();
  expect(find.text('Dashboard'), findsOneWidget);
});
```

### E2E Tests

**Coverage Target:** 40%

**Focus Areas:**
- User flows (login, crawl, results)
- Cross-platform (iOS, Android)
- Performance (launch time, memory)

## Test Environment

### Android

- **Emulator:** Pixel 6 API 33
- **Physical:** Samsung Galaxy S23
- **OS:** Android 13

### iOS

- **Simulator:** iPhone 14 Pro
- **Physical:** iPhone 14
- **OS:** iOS 16

## Test Automation

### CI/CD Integration

```yaml
# GitHub Actions
mobile-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: subosito/flutter-action@v2
      with:
        flutter-version: '3.16.0'
    - run: flutter pub get
    - run: flutter test
    - run: flutter test --coverage
    - run: flutter build apk --debug
```

### Testing Tools

- **flutter_test**: Unit and widget tests
- **integration_test**: Integration tests
- **patrol**: E2E tests
- **golden_toolkit**: Visual regression tests

## Test Data

### Mock Data

```dart
final mockCrawl = CrawlResponse(
  crawlId: 'test-123',
  status: 'completed',
  message: 'Crawl completed',
);
```

### Test Accounts

- **Admin:** admin@crawlkit.local / admin123
- **Editor:** editor@crawlkit.local / editor123
- **Viewer:** viewer@crawlkit.local / viewer123

## Performance Testing

### Launch Time

- **Target:** <2s cold start
- **Measurement:** `flutter run --profile` + timeline

### Memory Usage

- **Target:** <200MB idle
- **Measurement:** Xcode Instruments / Android Profiler

### Battery Usage

- **Target:** <5%/hour active
- **Measurement:** Battery historian / Xcode Energy Log

## Security Testing

### Authentication

- [ ] JWT token stored securely
- [ ] Biometric authentication works
- [ ] Token refresh works
- [ ] Logout clears tokens

### Data Protection

- [ ] No sensitive data in logs
- [ ] HTTPS enforced
- [ ] Certificate pinning works
- [ ] Input validation complete

## Accessibility Testing

### WCAG 2.1 AA

- [ ] Screen reader support
- [ ] Keyboard navigation
- [ ] Color contrast
- [ ] Touch target size

### Tools

- **Accessibility Scanner** (Android)
- **Accessibility Inspector** (iOS)
- **TalkBack** (Android)
- **VoiceOver** (iOS)

## Test Reporting

### Test Report Template

```markdown
# Mobile App Test Report

## Environment
- Date: YYYY-MM-DD
- Platform: Android/iOS
- Version: X.X.X

## Results

### Unit Tests
- Passed: X
- Failed: X
- Coverage: X%

### Widget Tests
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
- Launch time: Xms
- Memory usage: XMB
- Battery usage: X%/hour

### Issues
- [List any issues encountered]
```

## Continuous Testing

### Automated Testing

- Run unit tests on every commit
- Run widget tests on every PR
- Run integration tests nightly
- Run E2E tests weekly

### Manual Testing

- Test on real devices monthly
- Test new features before release
- Test accessibility quarterly
