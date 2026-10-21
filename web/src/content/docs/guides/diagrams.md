---
title: Architecture Diagrams
description: How the crawlkit docs render Mermaid diagrams with Pele.
---

Crawlkit's documentation renders Mermaid diagrams with
[Pele](https://pele.run) — a small, synchronous renderer with no runtime
dependencies and CSS-variable theming.

```mermaid
graph LR
    Q[URL Queue] --> DNS[DNS Prefetch]
    DNS --> F[Fetcher]
    F --> P[Parser]
    P --> A[Analyzers]
    A --> S[Storage]
    S --> E[Export]
    P --> LD[Link Discovery]
    LD --> Q
```

The loader is injected automatically on pages that contain diagrams;
colors follow the site theme via CSS variables.
