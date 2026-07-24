# Mobile Application Architecture

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.0.0 |
| Status | Draft |
| Author | Architecture Team |
| Last Updated | 2026-07-24 |
| Classification | Internal |

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Technology Selection Analysis](#2-technology-selection-analysis)
3. [Security Architecture](#3-security-architecture)
4. [Performance Requirements](#4-performance-requirements)
5. [Feature Specification](#5-feature-specification)
6. [Architecture Design](#6-architecture-design)
7. [Testing Strategy](#7-testing-strategy)
8. [Compliance Checklist](#8-compliance-checklist)
9. [Implementation Roadmap](#9-implementation-rodmap)
10. [Risk Assessment](#10-risk-assessment)
11. [Appendices](#11-appendices)

---

## 1. Executive Summary

This document defines the architecture for the CrawlKit mobile application, providing real-time web crawling monitoring, result visualization, and management capabilities. The application targets iOS and Android platforms with emphasis on security, performance, and enterprise-grade features.

### 1.1 Scope

- Cross-platform mobile application development
- Real-time crawl monitoring and management
- Offline-capable architecture
- Enterprise security and compliance
- Native platform integration

### 1.2 Objectives

- Deliver sub-2-second cold start performance
- Achieve 95%+ code sharing across platforms
- Implement OWASP Mobile Top 10 compliance
- Support enterprise multi-tenant deployments
- Provide full offline functionality

---

## 2. Technology Selection Analysis

### 2.1 Cross-Platform Framework Evaluation

#### 2.1.1 React Native

| Attribute | Assessment |
|-----------|------------|
| Language | JavaScript/TypeScript |
| Rendering | JavaScript bridge to native components |
| Hot Reload | Yes |
| Code Sharing | ~90% |
| Ecosystem | Large (npm) |
| Maintenance | Meta (Facebook) backed |

**Strengths:**
- Large developer talent pool
- Extensive third-party library ecosystem
- Strong community support
- Familiar web development paradigm

**Weaknesses:**
- Bridge performance bottleneck
- Inconsistent native feel
- Complex native module integration
- Dependency on third-party maintainers

#### 2.1.2 Flutter

| Attribute | Assessment |
|-----------|------------|
| Language | Dart |
| Rendering | Skia engine (custom rendering) |
| Hot Reload | Yes |
| Code Sharing | ~95% |
| Ecosystem | Growing (pub.dev) |
| Maintenance | Google backed |

**Strengths:**
- Consistent cross-platform UI
- Excellent performance (compiled to native ARM)
- Comprehensive widget library
- Single codebase for all platforms
- Strong typing and null safety

**Weaknesses:**
- Dart language adoption barrier
- Larger initial app size
- Platform-specific integrations require native code
- Web support still maturing

#### 2.1.3 Kotlin Multiplatform

| Attribute | Assessment |
|-----------|------------|
| Language | Kotlin |
| Rendering | Native UI per platform |
| Hot Reload | Limited |
| Code Sharing | ~70% (business logic) |
| Ecosystem | Growing (KMP ecosystem) |
| Maintenance | JetBrains backed |

**Strengths:**
- Native performance and feel
- Shared business logic
- Full access to platform APIs
- Modern language features

**Weaknesses:**
- No shared UI layer
- Smaller community
- Steeper learning curve
- More platform-specific code

#### 2.1.4 Xamarin / .NET MAUI

| Attribute | Assessment |
|-----------|------------|
| Language | C# |
| Rendering | Native via bindings |
| Hot Reload | Yes |
| Code Sharing | ~85% |
| Ecosystem | Medium (NuGet) |
| Maintenance | Microsoft backed |

**Strengths:**
- .NET ecosystem access
- Enterprise support
- Visual Studio integration
- Strong typing and tooling

**Weaknesses:**
- Larger app size
- Slower adoption rate
- Platform update lag
- Declining community activity

### 2.2 Recommendation Matrix

| Criteria | Weight | React Native | Flutter | Kotlin MP | Xamarin |
|----------|--------|-------------|---------|-----------|---------|
| Performance | 20% | 8/10 | 9/10 | 10/10 | 7/10 |
| Native Feel | 15% | 6/10 | 8/10 | 10/10 | 6/10 |
| Development Speed | 15% | 9/10 | 8/10 | 6/10 | 7/10 |
| Code Sharing | 15% | 9/10 | 10/10 | 7/10 | 8/10 |
| Community | 10% | 10/10 | 9/10 | 7/10 | 6/10 |
| Learning Curve | 10% | 9/10 | 7/10 | 6/10 | 8/10 |
| App Size | 5% | 7/10 | 7/10 | 9/10 | 5/10 |
| Testing | 10% | 8/10 | 8/10 | 8/10 | 7/10 |
| **Weighted Score** | **100%** | **8.15** | **8.50** | **7.70** | **6.85** |

### 2.3 Final Recommendation

**Primary Choice: Flutter**

Rationale:
1. Highest weighted score across all criteria
2. 95% code sharing reduces maintenance overhead
3. Compiled Dart delivers near-native performance
4. Consistent UI eliminates platform-specific design work
5. Strong Google backing ensures long-term viability
6. Excellent testing framework (flutter_test, integration_test)

**Secondary Choice: Kotlin Multiplatform**

Use cases:
- Applications requiring maximum native performance
- Teams with existing Kotlin expertise
- Projects with complex platform-specific requirements

---

## 3. Security Architecture

### 3.1 Threat Model

#### 3.1.1 Assets

| Asset | Sensitivity | Protection Level |
|-------|-------------|------------------|
| User credentials | Critical | Highest |
| API tokens | Critical | Highest |
| Crawl data | High | High |
| User PII | High | High |
| App configuration | Medium | Medium |
| Audit logs | High | High |

#### 3.1.2 Threat Actors

| Actor | Capability | Motivation |
|-------|-----------|------------|
| External attackers | Network interception | Data theft |
| Malicious insiders | Authorized access | Data exfiltration |
| Nation-state actors | Advanced persistent threats | Espionage |
| Competitors | Social engineering | IP theft |

### 3.2 Authentication Architecture

#### 3.2.1 Multi-Factor Authentication

```
┌─────────────────────────────────────────────────────────────┐
│                    Authentication Flow                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐              │
│  │  Primary  │───▶│ Secondary│───▶│  Token   │              │
│  │  Auth     │    │  Factor  │    │  Issue   │              │
│  └──────────┘    └──────────┘    └──────────┘              │
│       │               │               │                     │
│       ▼               ▼               ▼                     │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐              │
│  │Biometric │    │  TOTP    │    │   JWT    │              │
│  │/ Password│    │  / SMS   │    │  + Refresh│              │
│  └──────────┘    └──────────┘    └──────────┘              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.2 Biometric Authentication

**Implementation:**
- iOS: LocalAuthentication framework (Face ID, Touch ID)
- Android: BiometricPrompt API
- Fallback: Secure PIN/Pattern

**Requirements:**
- Hardware-backed keystore for biometric data
- Liveness detection enabled
- Max 5 failed attempts before lockout
- 30-second cooldown after 3 failures

#### 3.2.3 Token Management

**Access Token:**
- Algorithm: RS256
- Expiry: 15 minutes
- Claims: sub, iss, exp, iat, scope, tenant_id

**Refresh Token:**
- Algorithm: RS256
- Expiry: 7 days
- Rotation: On each use
- Storage: Secure enclave only

**Token Refresh Flow:**
```
1. Client detects 401 response
2. Client sends refresh token to /auth/refresh
3. Server validates refresh token
4. Server issues new access token
5. Server issues new refresh token
6. Client stores new tokens
7. Client retries original request
```

### 3.3 Data Protection

#### 3.3.1 Encryption Standards

| Data State | Algorithm | Key Size | Implementation |
|------------|-----------|----------|----------------|
| At rest | AES-256-GCM | 256-bit | AES-GCM |
| In transit | TLS 1.3 | 256-bit | Platform TLS |
| In memory | AES-256-CTR | 256-bit | Secure enclave |

#### 3.3.2 Key Management

**Key Hierarchy:**
```
Master Key (Hardware Security Module)
    ├── Key Encryption Key (KEK)
    │     ├── Data Encryption Key (DEK) - User data
    │     ├── Data Encryption Key (DEK) - Crawl data
    │     └── Data Encryption Key (DEK) - Cache
    └── Signing Key
          ├── JWT signing
          └── Request signing
```

**Key Storage:**
- iOS: Keychain Services (kSecAttrAccessibleWhenUnlockedThisDeviceOnly)
- Android: Android Keystore System (StrongBox if available)
- Never export keys from secure enclave

#### 3.3.3 Secure Data Handling

**Prohibited:**
- Sensitive data in application logs
- Sensitive data in crash reports
- Sensitive data in screenshots
- Clipboard persistence of sensitive data
- Keyboard cache for sensitive fields

**Required:**
- Clear sensitive data from memory after use
- Disable backup for sensitive data
- Screenshot prevention for sensitive screens
- Secure text entry for credentials

### 3.4 Network Security

#### 3.4.1 TLS Configuration

**Minimum Requirements:**
- TLS 1.3 (preferred) or TLS 1.2
- Strong cipher suites only
- Forward secrecy required

**Cipher Suites (TLS 1.3):**
```
TLS_AES_256_GCM_SHA384
TLS_CHACHA20_POLY1305_SHA256
TLS_AES_128_GCM_SHA256
```

**Certificate Pinning:**
- Pin leaf certificate public key
- Backup pins for certificate rotation
- Pin validation on every request
- Failure handling: Hard fail (no fallback)

#### 3.4.2 OAuth 2.0 / OIDC Implementation

**Authorization Code Flow with PKCE:**
```
1. Client generates code_verifier and code_challenge
2. Client redirects to authorization endpoint
3. User authenticates
4. Server returns authorization code
5. Client exchanges code + code_verifier for tokens
6. Server validates and issues tokens
```

**Token Rotation:**
- Refresh tokens rotate on each use
- Old refresh tokens invalidated immediately
- Token family tracking for breach detection

### 3.5 Compliance Requirements

#### 3.5.1 OWASP Mobile Top 10 Mitigations

| ID | Vulnerability | Mitigation |
|----|--------------|------------|
| M1 | Improper Platform Usage | Platform-specific security reviews |
| M2 | Insecure Data Storage | Encrypted storage, secure enclave |
| M3 | Insecure Communication | TLS 1.3, certificate pinning |
| M4 | Insecure Authentication | MFA, biometrics, token rotation |
| M5 | Insufficient Cryptography | AES-256-GCM, RS256 |
| M6 | Insecure Authorization | Server-side validation, RBAC |
| M7 | Client Code Quality | Static analysis, code review |
| M8 | Code Tampering | Obfuscation, integrity checks |
| M9 | Reverse Engineering | ProGuard/R8, certificate pinning |
| M10 | Extraneous Functionality | Feature flags, environment isolation |

#### 3.5.2 GDPR Compliance

| Requirement | Implementation |
|-------------|----------------|
| Data minimization | Collect only necessary data |
| Consent management | Granular consent UI |
| Right to access | Data export endpoint |
| Right to deletion | Account deletion flow |
| Data portability | JSON/CSV export |
| Data breach notification | 72-hour notification |

#### 3.5.3 CCPA Compliance

| Requirement | Implementation |
|-------------|----------------|
| Opt-out mechanism | Do Not Sell toggle |
| Data deletion | Deletion request flow |
| Privacy policy | In-app privacy center |
| Data collection disclosure | Transparent data usage |

---

## 4. Performance Requirements

### 4.1 Performance Targets

| Metric | Target | Measurement Method | Priority |
|--------|--------|-------------------|----------|
| Cold start | <2 seconds | App launch to interactive | Critical |
| Warm start | <1 second | App resume to interactive | Critical |
| Hot start | <500ms | App foreground to interactive | High |
| API response (p50) | <200ms | Client-side measurement | Critical |
| API response (p95) | <500ms | Client-side measurement | Critical |
| API response (p99) | <1000ms | Client-side measurement | High |
| Crawl progress update | <100ms | WebSocket latency | Critical |
| UI frame rate | 60 FPS | Frame rendering time | High |
| Scroll performance | <16ms | Frame time | High |
| Image load | <300ms | Time to first paint | Medium |

### 4.2 Resource Constraints

| Resource | Limit | Measurement |
|----------|-------|-------------|
| Memory (idle) | <200MB | Resident memory |
| Memory (active crawl) | <350MB | Resident memory |
| CPU (idle) | <5% | Average utilization |
| CPU (active crawl) | <25% | Average utilization |
| Battery usage | <5%/hour | Active crawl |
| Network usage | <50MB/hour | Active crawl |
| Storage (app) | <100MB | Installed size |
| Storage (cache) | <500MB | User-configurable |

### 4.3 Offline Support Requirements

**Cached Data:**
- Last 100 crawl results
- User preferences
- Authentication tokens
- Recent notifications
- App configuration

**Offline Capabilities:**
- View cached crawl results
- Start queued crawls (synced when online)
- Access settings and preferences
- View notification history

**Sync Strategy:**
- Conflict resolution: Last-write-wins
- Sync interval: 30 seconds (when online)
- Background sync: Every 15 minutes
- Manual sync: Pull-to-refresh

### 4.4 Performance Monitoring

**Metrics Collection:**
- App startup time
- Screen rendering time
- API response times
- Network request success/failure rates
- Memory usage patterns
- Battery impact
- Crash rates

**Alerting Thresholds:**
- Cold start >3 seconds
- API p95 >1000ms
- Memory >400MB
- Crash rate >1%
- ANR rate >0.1%

---

## 5. Feature Specification

### 5.1 Core Features

#### 5.1.1 Crawl Monitoring

**Real-time Progress Dashboard:**
- Active crawl status with progress percentage
- Page count and link discovery metrics
- Error count and warning indicators
- Elapsed time and estimated completion
- Visual progress indicator (linear/circular)

**Crawl History:**
- Chronological crawl list
- Search and filter capabilities
- Status indicators (completed, failed, running)
- Quick restart functionality

**Detail View:**
- Per-page crawl status
- Response time graphs
- Error details and stack traces
- URL hierarchy visualization

#### 5.1.2 Results Viewing

**Findings Display:**
- Categorized findings (errors, warnings, info)
- Severity indicators
- Affected URL display
- Recommendation text
- Trend analysis

**Report Generation:**
- PDF export
- CSV export
- Share functionality
- Scheduled report delivery

#### 5.1.3 Alert Notifications

**Push Notification Types:**
- Crawl completion
- Critical errors detected
- Threshold breaches
- Scheduled crawl reminders
- System maintenance notices

**Notification Management:**
- Per-crawl-type notification settings
- Quiet hours configuration
- Priority levels
- Notification history

#### 5.1.4 Offline Mode

**Cached Content:**
- Last 100 crawl results
- User preferences and settings
- Authentication state
- Recent notifications

**Offline Actions:**
- View cached results
- Queue new crawls
- Access settings
- View notification history

#### 5.1.5 Quick Actions

**Home Screen Actions:**
- Start new crawl
- View active crawls
- Access recent results
- Settings shortcut

**Widget Actions (iOS/Android):**
- Active crawl status
- Quick start crawl
- Recent result summary
- Error count badge

### 5.2 Advanced Features

#### 5.2.1 Voice Commands

**Supported Commands:**
- "Start crawl for [URL]"
- "Show active crawls"
- "Show crawl results"
- "Pause crawl"
- "Resume crawl"

**Integration:**
- iOS: Siri Shortcuts
- Android: Google Assistant Actions

#### 5.2.2 AR Visualization

**Site Structure Visualization:**
- 3D site map rendering
- Interactive node selection
- Relationship visualization
- Zoom and pan controls

**Requirements:**
- ARKit (iOS) / ARCore (Android)
- Device with AR capabilities
- Minimum iOS 12 / Android 8.0

#### 5.2.3 Widget Support

**iOS Widgets:**
- Small: Active crawl status
- Medium: Crawl progress with details
- Large: Dashboard summary

**Android Widgets:**
- App Widget: Crawl status
- Glance Widget: Quick actions
- Notification Widget: Active alerts

#### 5.2.4 Share Extension

**iOS Share Extension:**
- Share URL to CrawlKit
- Quick crawl initiation
- URL validation

**Android Share Intent:**
- Share URL to CrawlKit
- Quick crawl initiation
- URL validation

#### 5.2.5 Voice Assistant Integration

**Siri Shortcuts:**
- "Hey Siri, start a crawl"
- "Hey Siri, show my crawls"
- Custom shortcut creation

**Google Assistant Actions:**
- "Hey Google, start a crawl"
- "Hey Google, show crawl results"
- Custom actions

### 5.3 Enterprise Features

#### 5.3.1 Multi-Tenant Support

**Tenant Isolation:**
- Data isolation per tenant
- Tenant-specific configurations
- Resource quotas per tenant
- Tenant admin dashboard

**Tenant Management:**
- Tenant provisioning
- Tenant suspension/deletion
- Tenant configuration management
- Tenant usage monitoring

#### 5.3.2 Role-Based Access Control (RBAC)

**Roles:**
| Role | Permissions |
|------|-------------|
| Admin | Full access |
| Manager | Manage crawls, view results, manage users |
| Operator | Start/stop crawls, view results |
| Viewer | View results only |

**Permission Granularity:**
- Crawl management
- Results viewing
- User management
- Tenant settings
- Audit log access

#### 5.3.3 Audit Logging

**Logged Events:**
- Authentication events
- Crawl operations
- Data access
- Configuration changes
- User management actions

**Log Format:**
```json
{
  "timestamp": "ISO 8601",
  "event_type": "string",
  "actor": "user_id",
  "tenant": "tenant_id",
  "resource": "resource_type",
  "action": "action_type",
  "result": "success|failure",
  "details": {}
}
```

**Retention:**
- 90 days for operational logs
- 1 year for compliance logs
- Configurable per tenant

#### 5.3.4 SSO Integration

**Supported Protocols:**
- SAML 2.0
- OAuth 2.0 / OIDC
- WS-Federation

**Identity Providers:**
- Okta
- Azure AD
- Google Workspace
- Custom SAML providers

#### 5.3.5 Device Management

**Device Policies:**
- Jailbreak/root detection
- Minimum OS version
- Required security features
- Device compliance checks

**MDM Integration:**
- Mobile Device Management
- Device enrollment
- Policy enforcement
- Remote wipe capability

---

## 6. Architecture Design

### 6.1 System Architecture

#### 6.1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            Mobile Application                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Presentation Layer                          │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐   │   │
│  │  │  Screens  │  │  Widgets  │  │ Navigation│  │  Themes   │   │   │
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘   │   │
│  └────────┼──────────────┼──────────────┼──────────────┼───────────┘   │
│           │              │              │              │                │
│  ┌────────▼──────────────▼──────────────▼──────────────▼───────────┐   │
│  │                       Domain Layer                               │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐   │   │
│  │  │ Use Cases │  │ Entities  │  │Repositories│  │  Mapper   │   │   │
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘   │   │
│  └────────┼──────────────┼──────────────┼──────────────┼───────────┘   │
│           │              │              │              │                │
│  ┌────────▼──────────────▼──────────────▼──────────────▼───────────┐   │
│  │                        Data Layer                                │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐   │   │
│  │  │ API Client│  │   Cache   │  │ Database  │  │WebSocket  │   │   │
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘   │   │
│  └────────┼──────────────┼──────────────┼──────────────┼───────────┘   │
│           │              │              │              │                │
│  ┌────────▼──────────────▼──────────────▼──────────────▼───────────┐   │
│  │                      Platform Layer                              │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐   │   │
│  │  │Biometrics │  │ Keychain  │  │   Push    │  │  Device   │   │   │
│  │  │           │  │           │  │Notifcations│  │  Info     │   │   │
│  │  └───────────┘  └───────────┘  └───────────┘  └───────────┘   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 6.1.2 Component Architecture

**Presentation Layer Components:**
```
lib/
├── presentation/
│   ├── screens/
│   │   ├── login_screen.dart
│   │   ├── dashboard_screen.dart
│   │   ├── crawl_detail_screen.dart
│   │   ├── results_screen.dart
│   │   └── settings_screen.dart
│   ├── widgets/
│   │   ├── crawl_progress_widget.dart
│   │   ├── finding_card_widget.dart
│   │   ├── chart_widgets.dart
│   │   └── common_widgets.dart
│   ├── navigation/
│   │   └── app_router.dart
│   └── themes/
│       ├── app_theme.dart
│       └── app_colors.dart
```

**Domain Layer Components:**
```
lib/
├── domain/
│   ├── entities/
│   │   ├── crawl.dart
│   │   ├── finding.dart
│   │   ├── user.dart
│   │   └── tenant.dart
│   ├── repositories/
│   │   ├── crawl_repository.dart
│   │   ├── auth_repository.dart
│   │   └── user_repository.dart
│   └── usecases/
│       ├── start_crawl_usecase.dart
│       ├── get_crawl_results_usecase.dart
│       └── authenticate_usecase.dart
```

**Data Layer Components:**
```
lib/
├── data/
│   ├── models/
│   │   ├── crawl_model.dart
│   │   ├── finding_model.dart
│   │   └── user_model.dart
│   ├── datasources/
│   │   ├── remote/
│   │   │   ├── api_client.dart
│   │   │   └── websocket_client.dart
│   │   └── local/
│   │       ├── cache_service.dart
│   │       └── database_service.dart
│   └── repositories/
│       ├── crawl_repository_impl.dart
│       └── auth_repository_impl.dart
```

**Platform Layer Components:**
```
lib/
├── platform/
│   ├── biometrics/
│   │   ├── biometrics_service.dart
│   │   └── biometrics_service_impl.dart
│   ├── secure_storage/
│   │   ├── secure_storage_service.dart
│   │   └── secure_storage_service_impl.dart
│   ├── push_notifications/
│   │   ├── push_notification_service.dart
│   │   └── push_notification_service_impl.dart
│   └── device/
│       ├── device_info_service.dart
│       └── device_info_service_impl.dart
```

### 6.2 State Management

#### 6.2.1 BLoC Pattern

**Implementation:**
```dart
// CrawlBloc
class CrawlBloc extends Bloc<CrawlEvent, CrawlState> {
  final StartCrawlUseCase _startCrawlUseCase;
  final GetCrawlResultsUseCase _getCrawlResultsUseCase;
  
  CrawlBloc({
    required StartCrawlUseCase startCrawlUseCase,
    required GetCrawlResultsUseCase getCrawlResultsUseCase,
  }) : _startCrawlUseCase = startCrawlUseCase,
       _getCrawlResultsUseCase = getCrawlResultsUseCase,
       super(CrawlInitial()) {
    on<StartCrawl>(_onStartCrawl);
    on<LoadCrawlResults>(_onLoadCrawlResults);
    on<CrawlProgressUpdated>(_onCrawlProgressUpdated);
  }
}
```

**State Management Hierarchy:**
```
App-Level State
├── AuthBloc (authentication state)
├── ThemeBloc (theme state)
└── LocaleBloc (localization state)

Feature-Level State
├── CrawlBloc (crawl operations)
├── ResultsBloc (results viewing)
├── SettingsBloc (user settings)
└── NotificationBloc (notifications)
```

#### 6.2.2 Dependency Injection

**Provider Setup:**
```dart
MultiProvider(
  providers: [
    // Data Layer
    Provider<ApiClient>(create: (_) => ApiClientImpl()),
    Provider<CacheService>(create: (_) => CacheServiceImpl()),
    
    // Domain Layer
    Provider<CrawlRepository>(
      create: (context) => CrawlRepositoryImpl(
        apiClient: context.read<ApiClient>(),
        cacheService: context.read<CacheService>(),
      ),
    ),
    
    // Use Cases
    Provider<StartCrawlUseCase>(
      create: (context) => StartCrawlUseCase(
        crawlRepository: context.read<CrawlRepository>(),
      ),
    ),
    
    // BLoCs
    BlocProvider<CrawlBloc>(
      create: (context) => CrawlBloc(
        startCrawlUseCase: context.read<StartCrawlUseCase>(),
      ),
    ),
  ],
  child: MyApp(),
)
```

### 6.3 Navigation

#### 6.3.1 GoRouter Configuration

```dart
final router = GoRouter(
  redirect: (context, state) {
    final isLoggedIn = context.read<AuthBloc>().state is Authenticated;
    final isLoginRoute = state.matchedLocation == '/login';
    
    if (!isLoggedIn && !isLoginRoute) return '/login';
    if (isLoggedIn && isLoginRoute) return '/dashboard';
    return null;
  },
  routes: [
    GoRoute(
      path: '/login',
      builder: (context, state) => LoginScreen(),
    ),
    ShellRoute(
      builder: (context, state, child) => MainShell(child: child),
      routes: [
        GoRoute(
          path: '/dashboard',
          builder: (context, state) => DashboardScreen(),
        ),
        GoRoute(
          path: '/crawls',
          builder: (context, state) => CrawlsScreen(),
          routes: [
            GoRoute(
              path: ':id',
              builder: (context, state) => CrawlDetailScreen(
                crawlId: state.pathParameters['id']!,
              ),
            ),
          ],
        ),
        GoRoute(
          path: '/results',
          builder: (context, state) => ResultsScreen(),
        ),
        GoRoute(
          path: '/settings',
          builder: (context, state) => SettingsScreen(),
        ),
      ],
    ),
  ],
);
```

#### 6.3.2 Deep Linking

**iOS Configuration:**
```xml
<key>CFBundleURLTypes</key>
<array>
  <dict>
    <key>CFBundleURLSchemes</key>
    <array>
      <string>crawlkit</string>
    </array>
  </dict>
</array>
```

**Android Configuration:**
```xml
<intent-filter>
  <action android:name="android.intent.action.VIEW" />
  <category android:name="android.intent.category.DEFAULT" />
  <category android:name="android.intent.category.BROWSABLE" />
  <data
    android:host="app.crawlkit.io"
    android:scheme="https" />
</intent-filter>
```

### 6.4 Networking

#### 6.4.1 Dio HTTP Client

```dart
class ApiClientImpl implements ApiClient {
  late final Dio _dio;
  
  ApiClientImpl({
    required AuthService authService,
    required TokenService tokenService,
  }) {
    _dio = Dio(BaseOptions(
      baseUrl: 'https://api.crawlkit.io/v1',
      connectTimeout: Duration(seconds: 5),
      receiveTimeout: Duration(seconds: 15),
    ));
    
    _dio.interceptors.addAll([
      AuthInterceptor(authService, tokenService),
      LoggingInterceptor(),
      RetryInterceptor(maxRetries: 3),
      CertificatePinningInterceptor(),
    ]);
  }
}
```

#### 6.4.2 WebSocket Client

```dart
class WebSocketClientImpl implements WebSocketClient {
  WebSocketChannel? _channel;
  final StreamController<CrawlEvent> _controller = StreamController.broadcast();
  
  Stream<CrawlEvent> get events => _controller.stream;
  
  Future<void> connect(String crawlId) async {
    _channel = WebSocketChannel.connect(
      Uri.parse('wss://api.crawlkit.io/ws/crawls/$crawlId'),
    );
    
    _channel!.stream.listen(
      (data) {
        final event = CrawlEvent.fromJson(jsonDecode(data));
        _controller.add(event);
      },
      onError: (error) => _handleError(error),
      onDone: () => _handleDisconnect(),
    );
  }
}
```

### 6.5 Data Models

#### 6.5.1 Crawl Entity

```dart
@freezed
class Crawl with _$Crawl {
  const factory Crawl({
    required String id,
    required String url,
    required CrawlStatus status,
    required DateTime createdAt,
    DateTime? completedAt,
    int? totalPages,
    int? crawledPages,
    int? errorCount,
    List<Finding>? findings,
  }) = _Crawl;
}

enum CrawlStatus {
  pending,
  running,
  completed,
  failed,
  cancelled,
}
```

#### 6.5.2 Finding Entity

```dart
@freezed
class Finding with _$Finding {
  const factory Finding({
    required String id,
    required String crawlId,
    required String url,
    required FindingSeverity severity,
    required String category,
    required String message,
    String? recommendation,
    Map<String, dynamic>? metadata,
  }) = _Finding;
}

enum FindingSeverity {
  critical,
  high,
  medium,
  low,
  info,
}
```

### 6.6 Error Handling

#### 6.6.1 Error Categories

| Category | Examples | Handling |
|----------|----------|----------|
| Network | Timeout, DNS failure | Retry, offline fallback |
| Authentication | Invalid token, expired | Token refresh, re-login |
| Authorization | Insufficient permissions | Error message, upgrade prompt |
| Validation | Invalid input | Field-level errors |
| Server | 500, 502, 503 | Retry, error reporting |
| Client | Parsing, state | Log, crash reporting |

#### 6.6.2 Error Handling Strategy

```dart
sealed class AppError {
  const AppError();
  
  const factory AppError.network({String? message}) = NetworkError;
  const factory AppError.auth({String? message}) = AuthError;
  const factory AppError.permission({String? message}) = PermissionError;
  const factory AppError.validation({Map<String, String>? errors}) = ValidationError;
  const factory AppError.server({int? statusCode, String? message}) = ServerError;
}

class ErrorHandler {
  static AppError handle(Object error) {
    if (error is DioException) {
      return _handleDioError(error);
    }
    if (error is AuthException) {
      return AppError.auth(message: error.message);
    }
    return AppError.server(message: error.toString());
  }
}
```

---

## 7. Testing Strategy

### 7.1 Testing Pyramid

```
                    ┌─────────┐
                    │   E2E   │  5%
                    │  Tests  │
                    ├─────────┤
                    │Integration│  15%
                    │  Tests   │
                    ├─────────┤
                    │ Widget   │  30%
                    │  Tests   │
                    ├─────────┤
                    │  Unit   │  50%
                    │  Tests  │
                    └─────────┘
```

### 7.2 Unit Testing

**Coverage Targets:**
| Layer | Target | Critical Paths |
|-------|--------|----------------|
| Domain | 95% | 100% |
| Data | 90% | 95% |
| Presentation | 80% | 90% |
| Platform | 70% | 85% |

**Test Examples:**

```dart
// Use Case Test
group('StartCrawlUseCase', () {
  late StartCrawlUseCase useCase;
  late MockCrawlRepository mockRepository;
  
  setUp(() {
    mockRepository = MockCrawlRepository();
    useCase = StartCrawlUseCase(crawlRepository: mockRepository);
  });
  
  test('should return Crawl on success', () async {
    // Arrange
    when(mockRepository.startCrawl(any))
        .thenAnswer((_) async => testCrawl);
    
    // Act
    final result = await useCase(StartCrawlParams(url: 'https://example.com'));
    
    // Assert
    expect(result, equals(Right(testCrawl)));
    verify(mockRepository.startCrawl(any)).called(1);
  });
});
```

### 7.3 Widget Testing

```dart
// Widget Test
group('CrawlProgressWidget', () {
  testWidgets('should display progress percentage', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CrawlProgressWidget(
            progress: 0.75,
            totalPages: 100,
            crawledPages: 75,
          ),
        ),
      ),
    );
    
    expect(find.text('75%'), findsOneWidget);
    expect(find.text('75/100 pages'), findsOneWidget);
  });
});
```

### 7.4 Integration Testing

```dart
// Integration Test
group('Authentication Flow', () {
  testWidgets('should complete login flow', (tester) async {
    app.main();
    await tester.pumpAndSettle();
    
    // Enter credentials
    await tester.enterText(find.byKey(Key('email_field')), 'test@example.com');
    await tester.enterText(find.byKey(Key('password_field')), 'password123');
    await tester.tap(find.byKey(Key('login_button')));
    await tester.pumpAndSettle();
    
    // Verify dashboard
    expect(find.byType(DashboardScreen), findsOneWidget);
  });
});
```

### 7.5 E2E Testing

**Framework: Patrol (patrol.dev)**

```dart
// E2E Test
void main() {
  patrolTest('complete crawl workflow', (PatrolIntegrationTester $) async {
    await $.pumpAppAndSettle(MyApp());
    
    // Login
    await $.tester.enterText(find.byKey(Key('email')), 'test@example.com');
    await $.tester.enterText(find.byKey(Key('password')), 'password');
    await $.tester.tap(find.byKey(Key('login')));
    await $.pumpAndSettle();
    
    // Start crawl
    await $.tester.tap(find.byKey(Key('start_crawl_button')));
    await $.tester.enterText(find.byKey(Key('url_field')), 'https://example.com');
    await $.tester.tap(find.byKey(Key('confirm_crawl')));
    await $.pumpAndSettle();
    
    // Verify crawl started
    expect(find.byType(CrawlProgressWidget), findsOneWidget);
  });
}
```

### 7.6 Performance Testing

```dart
// Performance Test
void main() {
  testWidgets('scroll performance', (tester) async {
    await tester.pumpWidget(MaterialApp(home: ResultsScreen()));
    
    final stopwatch = Stopwatch()..start();
    
    // Perform scroll
    await tester.drag(
      find.byType(ListView),
      Offset(0, -500),
    );
    await tester.pump();
    
    stopwatch.stop();
    
    // Verify frame time < 16ms (60 FPS)
    expect(stopwatch.elapsedMilliseconds, lessThan(16));
  });
}
```

### 7.7 Security Testing

**OWASP MASVS Checklist:**
- [ ] Certificate pinning bypass attempts
- [ ] Token theft scenarios
- [ ] Root/jailbreak detection bypass
- [ ] Data extraction attempts
- [ ] Man-in-the-middle attacks
- [ ] Local storage encryption verification

### 7.8 Test Automation

**CI/CD Pipeline:**
```yaml
test:
  stages:
    - unit_tests
    - widget_tests
    - integration_tests
    - e2e_tests
    - performance_tests
    - security_tests
  
  coverage:
    threshold: 80%
    fail_on_threshold: true
  
  reports:
    - coverage_report.xml
    - test_results.xml
    - performance_report.json
```

---

## 8. Compliance Checklist

### 8.1 OWASP Mobile Top 10

| ID | Vulnerability | Status | Mitigation | Evidence |
|----|--------------|--------|------------|----------|
| M1 | Improper Platform Usage | [ ] | Platform security guidelines review | Security review doc |
| M2 | Insecure Data Storage | [ ] | Encrypted storage, secure enclave | Storage audit report |
| M3 | Insecure Communication | [ ] | TLS 1.3, certificate pinning | Network capture analysis |
| M4 | Insecure Authentication | [ ] | MFA, biometrics, token rotation | Auth flow documentation |
| M5 | Insufficient Cryptography | [ ] | AES-256-GCM, RS256 | Crypto implementation review |
| M6 | Insecure Authorization | [ ] | Server-side validation, RBAC | Authorization test results |
| M7 | Client Code Quality | [ ] | Static analysis, code review | Linting reports, review logs |
| M8 | Code Tampering | [ ] | Obfuscation, integrity checks | Tamper test results |
| M9 | Reverse Engineering | [ ] | ProGuard/R8, certificate pinning | Reverse engineering attempts |
| M10 | Extraneous Functionality | [ ] | Feature flags, environment isolation | Feature flag audit |

### 8.2 GDPR Compliance

| Requirement | Status | Implementation | Evidence |
|-------------|--------|----------------|----------|
| Data minimization | [ ] | Collect only necessary data | Data collection audit |
| Consent management | [ ] | Granular consent UI | Consent flow screenshots |
| Right to access | [ ] | Data export endpoint | API documentation |
| Right to deletion | [ ] | Account deletion flow | Deletion flow documentation |
| Data portability | [ ] | JSON/CSV export | Export feature testing |
| Data breach notification | [ ] | 72-hour notification process | Incident response plan |
| Privacy policy | [ ] | In-app privacy center | Privacy policy document |
| DPO appointment | [ ] | Designated DPO | DPO contact information |

### 8.3 CCPA Compliance

| Requirement | Status | Implementation | Evidence |
|-------------|--------|----------------|----------|
| Opt-out mechanism | [ ] | Do Not Sell toggle | Settings screen |
| Data deletion | [ ] | Deletion request flow | Deletion process documentation |
| Privacy policy | [ ] | Transparent data usage | Privacy policy document |
| Data collection disclosure | [ ] | What we collect section | Privacy policy documentation |
| Financial incentive disclosure | [ ] | N/A or disclosed | Privacy policy |

### 8.4 SOC 2 Compliance

| Criteria | Status | Implementation | Evidence |
|----------|--------|----------------|----------|
| Security | [ ] | Access controls, encryption | Security audit report |
| Availability | [ ] | SLA, monitoring | Uptime reports |
| Processing Integrity | [ ] | Validation, error handling | Processing tests |
| Confidentiality | [ ] | Data classification, encryption | Classification policy |
| Privacy | [ ] | Privacy controls | Privacy impact assessment |

### 8.5 HIPAA Compliance (if applicable)

| Requirement | Status | Implementation | Evidence |
|-------------|--------|----------------|----------|
| PHI encryption | [ ] | AES-256-GCM at rest/transit | Encryption verification |
| Access controls | [ ] | RBAC, audit logging | Access control documentation |
| Audit logging | [ ] | Comprehensive logging | Audit log review |
| Business Associate Agreement | [ ] | BAA with vendors | Signed BAAs |

---

## 9. Implementation Roadmap

### 9.1 Phase 1: Foundation (Weeks 1-4)

**Objective:** Establish project infrastructure and core architecture.

| Week | Deliverables | Dependencies |
|------|-------------|--------------|
| 1 | Project setup, CI/CD pipeline, repository structure | None |
| 2 | Authentication flow, secure storage, API client | Week 1 |
| 3 | Basic UI framework, navigation, theming | Week 2 |
| 4 | Dashboard screen, login flow, error handling | Week 3 |

**Milestones:**
- [ ] Flutter project initialized
- [ ] CI/CD pipeline operational
- [ ] Authentication flow complete
- [ ] Basic navigation working
- [ ] Secure storage implemented

### 9.2 Phase 2: Core Features (Weeks 5-10)

**Objective:** Implement primary application features.

| Week | Deliverables | Dependencies |
|------|-------------|--------------|
| 5 | Crawl monitoring, WebSocket integration | Phase 1 |
| 6 | Results viewing, finding display | Week 5 |
| 7 | Push notifications, alert management | Week 6 |
| 8 | Offline mode, caching strategy | Week 7 |
| 9 | Quick actions, home screen widgets | Week 8 |
| 10 | Feature integration, bug fixes | Week 9 |

**Milestones:**
- [ ] Crawl monitoring functional
- [ ] Results viewing complete
- [ ] Push notifications working
- [ ] Offline mode implemented
- [ ] Widgets functional

### 9.3 Phase 3: Advanced Features (Weeks 11-14)

**Objective:** Implement advanced platform-specific features.

| Week | Deliverables | Dependencies |
|------|-------------|--------------|
| 11 | Voice commands, Siri/Google Assistant | Phase 2 |
| 12 | AR visualization, site structure view | Week 11 |
| 13 | Share extension, deep linking | Week 12 |
| 14 | Advanced feature integration | Week 13 |

**Milestones:**
- [ ] Voice commands working
- [ ] AR visualization functional
- [ ] Share extension complete
- [ ] Deep linking operational

### 9.4 Phase 4: Enterprise Features (Weeks 15-18)

**Objective:** Implement enterprise-grade features.

| Week | Deliverables | Dependencies |
|------|-------------|--------------|
| 15 | Multi-tenant support, tenant isolation | Phase 3 |
| 16 | RBAC, permission system | Week 15 |
| 17 | Audit logging, SSO integration | Week 16 |
| 18 | Device management, MDM integration | Week 17 |

**Milestones:**
- [ ] Multi-tenant support complete
- [ ] RBAC functional
- [ ] Audit logging operational
- [ ] SSO integration working

### 9.5 Phase 5: Testing and Polish (Weeks 19-22)

**Objective:** Comprehensive testing and optimization.

| Week | Deliverables | Dependencies |
|------|-------------|--------------|
| 19 | Unit tests, widget tests | Phase 4 |
| 20 | Integration tests, E2E tests | Week 19 |
| 21 | Performance optimization, security testing | Week 20 |
| 22 | Bug fixes, documentation, release preparation | Week 21 |

**Milestones:**
- [ ] Test coverage >80%
- [ ] Performance targets met
- [ ] Security audit passed
- [ ] Documentation complete
- [ ] Release candidate ready

### 9.6 Timeline Summary

```
Week  1-4:   ████████████████████  Foundation
Week  5-10:  ████████████████████████████████  Core Features
Week 11-14:  ████████████████████  Advanced Features
Week 15-18:  ████████████████████  Enterprise Features
Week 19-22:  ████████████████████  Testing & Polish
             ─────────────────────────────────────────
             Total: 22 weeks
```

### 9.7 Resource Requirements

| Role | Count | Phase |
|------|-------|-------|
| Flutter Developer | 3 | All phases |
| Backend Developer | 2 | All phases |
| UI/UX Designer | 1 | Phase 1-3 |
| QA Engineer | 2 | Phase 5 |
| Security Engineer | 1 | Phase 4-5 |
| DevOps Engineer | 1 | All phases |

---

## 10. Risk Assessment

### 10.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Flutter framework limitations | Medium | High | Prototype critical features early |
| Platform-specific bugs | High | Medium | Extensive device testing |
| Performance issues | Medium | High | Performance budgets, profiling |
| Security vulnerabilities | Low | Critical | Security audits, OWASP compliance |
| API compatibility issues | Medium | Medium | API versioning, contract testing |

### 10.2 Project Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep | High | High | Strict feature prioritization |
| Resource constraints | Medium | High | Cross-training, documentation |
| Timeline delays | Medium | Medium | Buffer time, phased delivery |
| Team turnover | Low | High | Documentation, knowledge sharing |

### 10.3 Operational Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| App store rejection | Medium | High | Compliance review, guidelines adherence |
| API downtime | Low | High | Offline mode, retry logic |
| Data breach | Low | Critical | Security architecture, monitoring |
| Performance degradation | Medium | Medium | Performance monitoring, alerts |

---

## 11. Appendices

### 11.1 Glossary

| Term | Definition |
|------|------------|
| BLoC | Business Logic Component |
| RBAC | Role-Based Access Control |
| MFA | Multi-Factor Authentication |
| PKCE | Proof Key for Code Exchange |
| OIDC | OpenID Connect |
| TLS | Transport Layer Security |
| AES | Advanced Encryption Standard |
| JWT | JSON Web Token |
| SLA | Service Level Agreement |
| MDM | Mobile Device Management |

### 11.2 References

1. OWASP Mobile Security Testing Guide
2. OWASP MASVS (Mobile Application Security Verification Standard)
3. Flutter Documentation
4. Apple Human Interface Guidelines
5. Google Material Design Guidelines
6. GDPR Official Text
7. CCPA Official Text
8. SOC 2 Trust Services Criteria

### 11.3 Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2026-07-24 | Architecture Team | Initial draft |

---

**Document End**
