# Competitive Landscape

Last reviewed: 2026-10-14

## Positioning Thesis

Allie should not compete as another accessibility scanner. The proposed
differentiator is an evidence-packet harness: autonomous surface discovery,
generated replayable tests, WCAG criterion-by-surface visibility, artifact
drilldown, agentic context that remains non-blocking until promoted, and release
decisions that can be audited.

The reviewed products offer automation, guided manual review, dashboards,
AI assistance, and enterprise audit workflows. Allie should use those lessons
while staying sharper about local reproducibility, privacy policy, provenance,
and host-agnostic CI/release contracts.

The overlay trust gap is now part of the category context. In April 2025, the
FTC approved a final order requiring accessiBe to pay $1 million and barring
unsupported claims that automated products can make or keep websites
WCAG-compliant. Allie should not answer that market scar with stronger AI
claims. It should answer with owned evidence packets, criterion ledgers,
replay-to-revision provenance, and explicit residual review needs.

Each entry records the same fields so roadmap discussions can compare Allie
against scanner, dashboard, manual-audit, browser-cloud, design, and AI-assisted
alternatives without collapsing them into one vague category.

The record shape is machine-checked by `npm run landscape:smoke`. Keep the field
labels stable unless the script changes with the document contract. The script
checks structure, source presence, and freshness; it does not replace human or
agent review of vendor claims.

## Review Scope and Evidence Limits

The 2026-10-14 review inspected every previously cited primary-source URL and
followed relevant canonical documentation. The accompanying
[source ledger](landscape-source-review-2026-10-14.json) records checked URLs,
claims, outcomes, redirects, and retrieval limitations. Inspection dates are
not product-launch dates; public web extraction may use recently crawled text.

These records distinguish documented interfaces from vendor marketing. No
product account, live integration, coverage benchmark, or security audit was
used. Missing documentation means a capability was not established here, not
that it does not exist. Standards support is not complete automated coverage
or legal assurance. Privacy claims require product- and deployment-specific
validation. Allie differentiation notes describe strategic bets, not proof of
uniqueness or a claim that every target capability is implemented.

CI integration, exports, scripted states, guided replay, and AI assistance
already appear across the reviewed products. The defensible comparison is how
evidence preserves revision identity, explicit gaps, promotion decisions, and
release policy across tools. Existing roadmap priorities remain; this review
does not authorize integrations, provider calls, or feature expansion.

## Landscape Records

### Deque axe-core

- **Category:** Open-source accessibility rules engine.
- **Automation depth:** README documents browser, unit, integration, and
  functional-test use; callers choose which rendered states to scan.
- **Standards claims:** Rules cover subsets of WCAG 2.0, 2.1, and 2.2 across
  A/AA/AAA, plus best practices. This is not complete conformance coverage.
- **CI/PR integration:** Integrates into existing automated tests; the calling
  harness decides failures and CI policy.
- **Evidence artifacts and replay:** API documents violations, passes,
  incomplete and inapplicable results, rule tags, and element targets. Browser
  state reconstruction and revision provenance remain harness responsibilities.
- **Manual review workflow:** Incomplete results explicitly identify cases
  needing human review; axe-core does not provide a guided audit interface.
- **AI/agentic claims:** No AI feature is documented for the core engine in the
  reviewed sources; commercial Axe products are a separate comparison.
- **Privacy/governance posture:** Locally bundled JavaScript can run in the
  consumer's test environment; surrounding integrations need separate review.
- **Allie differentiation:** Proposed advantage is the evidence and release
  contract around engine output, not scanner ownership or full WCAG coverage.
- **Sources:** [axe-core GitHub](https://github.com/dequelabs/axe-core),
  [Deque axe-core](https://www.deque.com/axe/axe-core/),
  [axe-core API](https://github.com/dequelabs/axe-core/blob/develop/doc/API.md)

### accessiBe and accessibility overlays

- **Category:** accessWidget overlay, within a vendor portfolio that also
  advertises developer tooling and expert services; not all overlays are alike.
- **Automation depth:** Vendor markets AI-driven page adjustments and recurring
  scans. These are product claims, not independently tested effectiveness.
- **Standards claims:** The April 2025 FTC order requires substantiation for
  automated WCAG-compliance claims and $1 million payment. It concerns accessiBe,
  not a finding about every overlay. Current marketing names WCAG 2.2 AA while
  its FAQ still references 2.1 AA and acknowledges remediation limitations.
- **CI/PR integration:** The reviewed accessWidget page describes website/CMS
  installation; it does not establish CI behavior for the separate accessFlow.
- **Evidence artifacts and replay:** Vendor advertises audit and impact reports;
  a per-revision replay contract was not established in the reviewed sources.
- **Manual review workflow:** Vendor now advertises expert flow testing,
  assistive-technology checks, and documented custom enhancements.
- **AI/agentic claims:** AI remediation and a visitor-facing assistant are
  advertised; effectiveness was not tested in this review.
- **Privacy/governance posture:** Vendor asserts SOC 2-related controls; this
  review did not verify data flows, retention, or audit reports.
- **Allie differentiation:** Prioritize substantiated evidence and residual
  review needs; avoid automated legal-compliance promises.
- **Sources:** [accessWidget](https://accessibe.com/accesswidget),
  [FTC final-order announcement](https://www.ftc.gov/news-events/news/press-releases/2025/04/ftc-approves-final-order-requiring-accessibe-pay-1-million),
  [FTC accessiBe case page](https://www.ftc.gov/legal-library/browse/cases-proceedings/2223156-accessibe-inc)

### Deque axe DevTools and axe Platform

- **Category:** Developer testing, guided/manual evaluation, and monitoring suite.
- **Automation depth:** Combines automated checks, Intelligent Guided Tests
  (IGTs), and platform workflows; feature availability depends on product/tier.
- **Standards claims:** WCAG-oriented testing claims do not establish complete
  automated coverage or a legal guarantee.
- **CI/PR integration:** Deque explicitly markets Watcher/API pipeline testing,
  PR linting, and automated blocking; this is more than Jira integration.
- **Evidence artifacts and replay:** Extension marketing lists JSON, CSV, JUnit
  XML, screenshots, and permalinks. IGT documentation explicitly supports Auto
  Replay for Modal Dialog, Keyboard, and Forms, reusing answers and checking
  changes; this is not evidence that all tests are replayable.
- **Manual review workflow:** IGTs ask structured questions and retain progress;
  Axe Auditor provides a separate manual-testing workflow.
- **AI/agentic claims:** AI-assisted testing and Axe MCP Server remediation in AI
  coding environments are marketed; no hands-on efficacy test was performed.
- **Privacy/governance posture:** MCP marketing asserts enterprise security;
  the IGT documentation warns against personal-data use. Deployment and
  data-handling controls require product-specific verification.
- **Allie differentiation:** Validate revision-linked, portable evidence and
  explicit promotion policy as the advantage; replay alone is not unique.
- **Sources:** [axe Platform](https://www.deque.com/axe/),
  [axe DevTools Extension](https://www.deque.com/axe/devtools/extension/),
  [IGT and Auto Replay docs](https://docs.deque.com/devtools-for-web/4/en/devtools-igt/),
  [CI/CD integration](https://www.deque.com/accessible-development/ci-cd/),
  [Axe MCP Server](https://www.deque.com/axe/mcp-server/)

### Evinced

- **Category:** Accessibility automation platform with web/mobile SDKs and AI tools.
- **Automation depth:** Vendor describes visual/semantic analysis, DOM-change
  scanning, clustering, and issue tracking; Playwright documentation specifies
  integration into existing functional tests.
- **Standards claims:** Reports associate findings with WCAG tags. Marketing
  claims broader detection; this review does not establish full WCAG coverage.
- **CI/PR integration:** SDK marketing supports configurable build failures by
  issue type/severity; Playwright integration is publicly documented.
- **Evidence artifacts and replay:** Public report schema includes IDs,
  signatures, severity, DOM snippets, selectors, and WCAG tags; JSON, HTML, and
  CSV examples are published. Functional-test reruns are supported; a portable
  revision-linked evidence contract was not established.
- **Manual review workflow:** Vendor guidance treats automation as augmentation
  of human assessment, not a replacement for all expert testing.
- **AI/agentic claims:** Detection/clustering and MCP coding tools are marketed;
  SDK docs also describe packaged AI-assistant integration skills.
- **Privacy/governance posture:** Playwright docs describe online or supplied-token
  offline authentication and optional uploads. Enterprise AI marketing describes
  hosted/on-premise options and customizable retention; scope varies by product.
- **Allie differentiation:** Test the value of explicit evidence promotion and
  revision provenance; local output, SDK CI integration, and AI are not unique.
- **Sources:** [Evinced](https://www.evinced.com/),
  [Technology](https://www.evinced.com/technology),
  [Automation overview](https://www.evinced.com/blog/introduction-to-test-automation-for-accessibility-managers/),
  [Automation SDKs](https://www.evinced.com/products/auto-test-sdks),
  [Web report schema](https://developer.evinced.com/sdks-for-web-apps/web-reports/),
  [Playwright SDK docs](https://developer.evinced.com/sdks-for-web-apps/playwright-js-sdk/),
  [Enterprise AI](https://www.evinced.com/ai/enterprises),
  [MCP tools](https://www.evinced.com/products/mcp-tools)

### Level Access

- **Category:** Enterprise accessibility platform, developer tooling, and services.
- **Automation depth:** Markets scanning, monitoring, expert evaluation,
  remediation support, and program reporting.
- **Standards claims:** Platform statement names support for WCAG 2 A/AA,
  Section 508, and EN 301 549; Level CI separately markets WCAG 2.0/2.1 rules
  through AAA. Neither establishes complete automated conformance coverage.
- **CI/PR integration:** Level CI explicitly advertises pre-merge checks,
  severity/score quality gates, delta scans, and pull-request fix recommendations.
- **Evidence artifacts and replay:** Markets deduplicated delta reports,
  monitoring, and audit reporting. Portable local replay/provenance semantics
  were not established by the reviewed pages.
- **Manual review workflow:** Expert audits and user testing complement
  automated scans; the testing guide says human assessment remains necessary.
- **AI/agentic claims:** AI agents are an explicit current offering, including
  reporting, monitoring summaries, natural-language filtering, and fix guidance.
- **Privacy/governance posture:** Vendor states security/privacy controls,
  human oversight, SOC 2 Type II audits, ISO 27001 certification, and no sensitive
  data processing by its AI; these assertions were not independently audited.
- **Allie differentiation:** Compare portable evidence and provenance directly
  against Level CI; entering the developer/CI loop is not a unique advantage.
- **Sources:** [Level Access](https://www.levelaccess.com/),
  [Accessibility statement](https://www.levelaccess.com/accessibility-statement/),
  [Automated testing guide](https://www.levelaccess.com/blog/automated-accessibility-testing-a-practical-guide-to-wcag-coverage/),
  [Level CI](https://www.levelaccess.com/level-ci/),
  [AI agents](https://www.levelaccess.com/ai-accessibility-agents/)

### Accessibility Insights

- **Category:** Open-source browser extension with guided assessments.
- **Automation depth:** FastPass combines axe-powered checks and assisted keyboard
  testing; hidden states must be revealed before scanning.
- **Standards claims:** The Assessment guide now targets WCAG 2.2 AA; the overview
  still says 2.1 AA. Automated checks provide partial criterion coverage.
- **CI/PR integration:** The FAQ links axe pipeline samples; full FastPass and
  Assessment require human analysis and cannot be completely automated.
- **Evidence artifacts and replay:** Assessment exports HTML and criterion-grouped
  JSON and saves resumable assessment files. The FAQ says results cannot be
  aggregated across pages or states; saved assessments are not browser replay.
- **Manual review workflow:** Automated, assisted, and manual requirements have
  pass/fail/incomplete states and step-by-step guidance.
- **AI/agentic claims:** No AI assessment capability identified in the reviewed
  product documentation.
- **Privacy/governance posture:** FAQ says complete scan data stays in the browser;
  opt-in telemetry includes violation counts, excluding tested URLs and snippets.
- **Allie differentiation:** Proposed advantage: connect multiple states and
  revisions to replay and release evidence while retaining guided human review.
- **Sources:** [Overview](https://accessibilityinsights.io/docs/web/overview/),
  [Assessment](https://accessibilityinsights.io/docs/web/getstarted/assessment/),
  [FAQ](https://accessibilityinsights.io/docs/web/reference/faq/)

### Pa11y

- **Category:** Open-source CLI, CI, dashboard, and webservice family.
- **Automation depth:** CLI/Node scanning uses HTML_CodeSniffer or axe; documented
  scripted actions, headers, and supplied browser instances support prepared states.
- **Standards claims:** Configurable WCAG levels and runner-specific rules;
  Pa11y CI explicitly says passing automation does not establish accessibility
  or standards conformance.
- **CI/PR integration:** Pa11y CI scans URL lists, sitemaps, and local files;
  configurable thresholds and exit codes support build decisions.
- **Evidence artifacts and replay:** Structured results, JSON/CSV reporters,
  scripted actions, and optional screenshots are documented. Repeatable scripts
  should not be mistaken for an independently verified provenance contract.
- **Manual review workflow:** Automation complements manual and assistive-technology
  testing; no guided assessment workflow identified in these sources.
- **AI/agentic claims:** No AI detection claim identified in the reviewed project
  documentation.
- **Privacy/governance posture:** Local/CI operation is supported; deployment,
  reporters, and target-page requests determine the wider data boundary.
- **Allie differentiation:** Proposed advantage: unify scripted-state evidence,
  criterion coverage, human decisions, and revision provenance rather than claim
  authenticated interaction or CI thresholds are unique.
- **Sources:** [Pa11y](https://pa11y.org/),
  [Pa11y GitHub](https://github.com/pa11y/pa11y),
  [Pa11y CI](https://github.com/pa11y/pa11y-ci)

### Lighthouse and Chrome DevTools audits

- **Category:** Open-source developer audit runner with reports and scores.
- **Automation depth:** DevTools/CLI/Node audits; scripted user flows include
  accessibility snapshots. Timespan mode has no accessibility category.
- **Standards claims:** Accessibility scoring weights automated audits using axe
  impact assessments; manual and low-impact/best-practice checks are excluded
  from the score. A score is not a full WCAG conformance finding.
- **CI/PR integration:** Lighthouse CI runs, stores, retrieves, and asserts against
  results for regression workflows.
- **Evidence artifacts and replay:** HTML/JSON reports, saved traces/logs, and
  separate gathering/re-auditing modes are documented, alongside flow reports.
- **Manual review workflow:** Reports provide remediation references and manual
  checks; these do not constitute a complete guided WCAG assessment.
- **AI/agentic claims:** Chrome documents coding agents invoking Lighthouse and
  interpreting accessibility findings. Separate experimental Agentic Browsing
  audits use deterministic signals for machine interaction, not human WCAG proof.
- **Privacy/governance posture:** Core audits run locally; optional error reporting,
  report sharing, hosted CI, and agent integrations require separate consideration.
- **Allie differentiation:** Proposed advantage: criterion-by-surface provenance
  and human-attested release decisions beyond scores, flow reports, and re-audits.
- **Sources:** [Lighthouse overview](https://developer.chrome.com/docs/lighthouse/overview),
  [Accessibility scoring](https://developer.chrome.com/docs/lighthouse/accessibility/scoring),
  [Lighthouse GitHub](https://github.com/GoogleChrome/lighthouse),
  [User flows](https://github.com/GoogleChrome/lighthouse/blob/main/docs/user-flows.md),
  [Lighthouse CI](https://github.com/GoogleChrome/lighthouse-ci),
  [Agent workflows](https://developer.chrome.com/docs/devtools/agents/use-cases/lighthouse-audit),
  [Agentic Browsing scoring](https://developer.chrome.com/docs/lighthouse/agentic-browsing/scoring)

### BrowserStack Accessibility

- **Category:** Browser/device-cloud accessibility suite with scanning, assisted
  tests, monitoring, and screen-reader workflows.
- **Automation depth:** Current marketing describes Spectra and AI-assisted
  detection, rather than only axe-core. Documentation describes workflow scans
  and scheduled website monitoring.
- **Standards claims:** Vendor says scanners target WCAG 2.0, 2.1, and 2.2 at
  A, AA, and AAA; this is an issue-detection claim, not proof of full conformance.
- **CI/PR integration:** SDK-based CI/CD checks, DOM-change triggers, configurable
  scan scopes, and assertions are advertised; not exercised in this review.
- **Evidence artifacts and replay:** Vendor describes centralized reports,
  deduplicated findings, build history, and conformance summaries. A portable
  replay-packet contract was not established by the reviewed sources.
- **Manual review workflow:** Docs describe Assisted Tests with user confirmation
  and native VoiceOver, NVDA, and TalkBack testing.
- **AI/agentic claims:** Markets contextual issue detection and remediation
  guidance; accuracy and coverage figures are vendor claims, not benchmarks here.
- **Privacy/governance posture:** Web-based reporting and cloud-device workflows
  are documented. Retention, model routing, and redaction guarantees were not
  verified in these sources.
- **Allie differentiation:** Make cloud execution optional while preserving
  portable provenance and replay. Do not imply BrowserStack lacks release
  assertions or visibility into remaining manual checks.
- **Sources:** [BrowserStack accessibility testing](https://www.browserstack.com/accessibility-testing),
  [Accessibility overview docs](https://www.browserstack.com/docs/accessibility/overview/introduction),
  [Automated tests](https://www.browserstack.com/accessibility-testing/features/automated-tests),
  [BrowserStack AI agents](https://www.browserstack.com/accessibility-testing/ai-agents)

### Sauce Labs Accessibility

- **Category:** Testing cloud with Deque accessibility integrations.
- **Automation depth:** Docs show axe-core in browser tests; product marketing
  separately describes axe DevTools Mobile for Appium on the Real Device Cloud.
- **Standards claims:** Marketing frames the partnership around EAA readiness.
  The reviewed integration guide does not establish exhaustive WCAG-version or
  success-criterion coverage, or guarantee compliance.
- **CI/PR integration:** Documented examples add axe checks to Selenium and
  WebdriverIO tests; the Deque guide demonstrates scanning the current page.
- **Evidence artifacts and replay:** Sauce docs describe an Accessibility tab
  and detailed full-page results. Cross-tool replay provenance and release
  packet semantics were not established by these sources.
- **Manual review workflow:** The reviewed integration material focuses on
  automated tests and result inspection; it does not specify a guided manual
  WCAG assessment workflow.
- **AI/agentic claims:** No accessibility-specific AI capability was established
  by these three sources; broader Sauce AI navigation is not sufficient evidence.
- **Privacy/governance posture:** Tests use a Sauce account and cloud
  infrastructure; the Deque example exposes region selection. Accessibility-data
  retention and AI-processing guarantees were not verified here.
- **Allie differentiation:** Support Sauce as an execution adapter while keeping
  the evidence schema and release policy portable.
- **Sources:** [Sauce accessibility testing](https://saucelabs.com/products/accessibility-testing),
  [Sauce Deque integration docs](https://docs.saucelabs.com/basics/integrations/deque/),
  [Deque and Sauce guide](https://www.deque.com/saucelabs/get-started/)

### Stark

- **Category:** Accessibility platform spanning design, source code, live assets,
  reporting, and compliance management.
- **Automation depth:** Marketing describes continuous design/repository scans;
  support docs specify CLI scanning, ESLint integration, and dashboard sync.
- **Standards claims:** WCAG-oriented issue detection and regulatory-control
  management are advertised; the reviewed pages do not establish complete
  criterion coverage or automatic legal compliance.
- **CI/PR integration:** Source-code docs provide token-authenticated CLI scans
  and CI/CD configuration; the product also advertises GitHub integration.
- **Evidence artifacts and replay:** Project reports and audit evidence are
  marketed; CLI findings synchronize to Stark. A portable runtime-replay packet
  contract was not established by these sources.
- **Manual review workflow:** Compliance Center lets teams assign/document
  controls and access evidence. This is governance workflow, not verified
  exhaustive manual test coverage.
- **AI/agentic claims:** Explicit AI suggestions and automation; an August 2026
  announcement describes a Claude connector for scans, findings, and compliance
  status. These are vendor-described capabilities, not tested here.
- **Privacy/governance posture:** Marketing names SSO and SOC 2; source-code docs
  require a token and automatically upload scan results. Certification scope,
  retention, and model-routing guarantees were not independently verified.
- **Allie differentiation:** Prioritize reproducible runtime evidence and explicit
  promotion into release gates; do not position Stark as design-only or merely
  offering smart suggestions.
- **Sources:** [Stark](https://www.getstark.co/),
  [Source code scanning](https://www.getstark.co/blog/source-code-scanning/),
  [Source-code integration docs](https://www.getstark.co/support/getting-started/using-source-code-integration/),
  [Claude connector announcement](https://www.getstark.co/blog/claude-connector/)

### WAVE and WebAIM

- **Category:** Online evaluation tool, browser extensions, subscription API, and
  licensable stand-alone engine.
- **Automation depth:** Page checks through hosted, extension, and API workflows;
  scripting support differs, so identical URLs need not produce identical results.
- **Standards claims:** Help documents WCAG 2.2/Section 508-related checks and
  explicitly rejects treating an error-free result as accessibility certification.
- **CI/PR integration:** WebAIM documents stand-alone infrastructure and CI
  integration; the subscription API performs remote URL analysis.
- **Evidence artifacts and replay:** In-page indicators and JSON/XML API results,
  optionally with XPath detail; browser-state replay is the caller's responsibility.
- **Manual review workflow:** Visual indicators, code inspection, and guidance
  support human evaluation, including matters automation cannot determine.
- **AI/agentic claims:** No AI evaluation claim identified in the reviewed WAVE
  documentation.
- **Privacy/governance posture:** WebAIM says extension evaluation stays in the
  browser without sending information to its servers. This does not describe the
  hosted URL/API service; stand-alone deployment is a separate option.
- **Allie differentiation:** Proposed advantage: connect scanner results and
  human judgments to reproducible product states and release provenance.
- **Sources:** [WAVE](https://wave.webaim.org/),
  [WAVE API](https://wave.webaim.org/api/),
  [WAVE extension](https://wave.webaim.org/extension/),
  [WAVE Help](https://wave.webaim.org/help)

### Siteimprove

- **Category:** Enterprise accessibility and content-governance platform,
  currently marketed as Siteimprove.ai.
- **Automation depth:** Marketing describes continuous monitoring,
  prioritization, and remediation across public/authenticated digital assets;
  breadth and effectiveness were not tested.
- **Standards claims:** Current pages discuss WCAG and ADA/Section 508/EAA
  support. The formerly cited product URL redirects to a page that does not
  substantiate the previous exact WCAG-version/level coverage claim.
- **CI/PR integration:** Current marketing explicitly names CI/CD, IDE/design
  workflows, and an MCP Server. Integration behavior was not exercised.
- **Evidence artifacts and replay:** Help content documents DCI Accessibility
  score changes when new rules are enabled; reviewed pages did not establish
  portable revision-bound replay artifacts.
- **Manual review workflow:** Vendor explicitly offers certified-expert manual
  testing, consulting, education, VPATs, and training alongside automation.
- **AI/agentic claims:** Marketing promotes accessibility agents; help documents
  opt-in AI-supported checks for descriptive page titles and headings. Its
  statement of three new rules conflicts with the two rules actually listed.
- **Privacy/governance posture:** Marketing describes shared platform governance;
  deployment, retention, and model data flows were not verified.
- **Allie differentiation:** Focus on independently inspectable release evidence;
  CI integration, agents, and governance are already competitor claims.
- **Sources:** [Accessibility platform](https://www.siteimprove.com/platform/accessibility/),
  [Compliance overview](https://www.siteimprove.com/web-accessibility-compliance/),
  [AI-supported rules](https://help.siteimprove.com/support/solutions/articles/80001183313-expanding-wcag-coverage-with-new-ai-supported-rules)

### IBM Equal Access Toolkit

- **Category:** Open accessibility lifecycle guidance and checker tooling.
- **Automation depth:** Browser extensions and Node testing packages accompany
  plan/design/develop/verify/launch guidance.
- **Standards claims:** Repository topics identify WCAG 2.0/2.1/2.2 and US Section
  508; IBM explicitly says automated checks cover only a subset of requirements.
- **CI/PR integration:** Node tooling documents Selenium, Puppeteer, Playwright,
  and CI integration, configurable failure levels, and baseline comparisons.
- **Evidence artifacts and replay:** JSON/CSV/HTML results, optional XLSX, rule
  archive selection, and baselines are documented. These are substantive
  reproducibility controls, not a tested end-to-end release provenance guarantee.
- **Manual review workflow:** IBM requires manual and screen-reader tests after
  automated verification; results distinguish violations and items needing review.
- **AI/agentic claims:** No AI checking claim identified in the reviewed toolkit
  and checker documentation.
- **Privacy/governance posture:** Browser and local test tooling are available;
  Node docs reference externally loaded engines/rule servers, so local execution
  alone does not establish an offline or zero-egress configuration.
- **Allie differentiation:** Proposed advantage: connect existing rule/version
  controls and review outcomes into portable surface-to-release provenance.
- **Sources:** [IBM Equal Access Toolkit](https://www.ibm.com/able/toolkit/),
  [Verify automated](https://www.ibm.com/able/toolkit/verify/automated/),
  [Equal Access GitHub](https://github.com/ibma/equal-access),
  [Node checker documentation](https://github.com/IBMa/equal-access/blob/main-4.x/accessibility-checker/README.md)

### W3C WAI tools list and ACT Rules

- **Category:** Evaluation-tool directory and accessibility testing-rule resources.
- **Automation depth:** ACT addresses automated, semi-automated, and manual
  methods; the directory is not a testing engine.
- **Standards claims:** ACT Rules cover aspects of WCAG 2.2, ARIA, and other
  practices. They are informative, partial checks; WCAG success criteria remain
  normative, and passing rules does not establish full conformance.
- **CI/PR integration:** Implementers incorporate rules into tools and
  methodologies; W3C does not supply a CI runner through these resources.
- **Evidence artifacts and replay:** Rules include applicability, expectations,
  mappings, and passed/failed/inapplicable examples for checking implementation
  consistency, rather than product-specific execution evidence.
- **Manual review workflow:** Manual and semi-automated implementation types are
  explicit; broader criterion verification can require human testing.
- **AI/agentic claims:** No AI product capability is claimed by these standards
  resources; listed products' claims belong to their submitters.
- **Privacy/governance posture:** W3C disclaims endorsement of provider-submitted
  tool listings; the rules list distinguishes approved, proposed, and deprecated
  rules. Runtime privacy depends on the implementing tool.
- **Allie differentiation:** Proposed use: record rule status, applicability, and
  uncovered criterion aspects instead of converting rule passes into compliance.
- **Sources:** [W3C evaluation tools list](https://www.w3.org/WAI/test-evaluate/tools/list/),
  [ACT overview](https://www.w3.org/WAI/standards-guidelines/act/),
  [ACT Rules](https://www.w3.org/WAI/standards-guidelines/act/rules/),
  [About ACT Rules](https://www.w3.org/WAI/standards-guidelines/act/rules/about/)

### TPGi ARC Platform and ARC Toolkit

- **Category:** Enterprise accessibility monitoring and a page-testing extension,
  now presented on Vispero's site.
- **Automation depth:** ARC markets continuous scans and regression monitoring;
  Toolkit performs on-demand single-page checks using the ARC rules engine.
- **Standards claims:** ARC monitoring names WCAG 2.2 and regulatory alignment;
  the separate Toolkit page still specifies WCAG 2.1 A/AA. These claims must not
  be conflated into identical coverage.
- **CI/PR integration:** Current pages describe development/QA integration and
  mention the ARC API; they do not establish specific CI gate or PR mechanics.
- **Evidence artifacts and replay:** Dashboards, trends, and remediation tracking
  are advertised; Toolkit exposes element-level findings and copy/paste into
  issue trackers. Portable replay semantics were not established.
- **Manual review workflow:** Vispero explicitly says monitoring does not replace
  expert manual testing. Toolkit provides tab-order visualization and detailed
  on-page inspection.
- **AI/agentic claims:** No AI-specific capability established in the reviewed ARC
  and Toolkit pages; governance intelligence wording alone is insufficient.
- **Privacy/governance posture:** Enterprise governance/reporting is marketed;
  data residency, retention, and local-only processing were not verified here.
- **Allie differentiation:** Keep evidence and release decisions portable across
  monitoring vendors; distinguish engineering provenance from dashboard trends.
- **Sources:** [ARC monitoring](https://vispero.com/accessibility-audit-vpat-testing/arc/),
  [ARC Toolkit](https://vispero.com/lp/arc-toolkit/)

### Applitools Contrast Advisor

- **Category:** Eyes visual-testing feature for contrast analysis.
- **Automation depth:** Documentation describes pixel-based contrast checks at
  test checkpoints, including images, icons, and non-DOM mobile content.
- **Standards claims:** Docs identify WCAG 1.4.3, 1.4.6, and 1.4.11; product
  marketing offers WCAG 2.0/2.1 with AA/AAA settings. This is contrast coverage,
  not a full accessibility assessment.
- **CI/PR integration:** Enabled in existing Eyes SDK test runs; specific
  accessibility PR-gate behavior was not verified in the reviewed pages.
- **Evidence artifacts and replay:** Test Manager shows checkpoint images,
  measured/required ratios, and separate accessibility status. Contrast checks
  explicitly do not compare against a reference visual baseline.
- **Manual review workflow:** Users inspect flagged regions and may override
  element classification or exclude regions; this is contrast-result review.
- **AI/agentic claims:** Markets Visual AI for contrast measurement; no autonomous
  whole-WCAG assurance follows from that claim.
- **Privacy/governance posture:** Current product marketing claims local visual
  processing and no external-model transmission or training of snapshots.
  Deployment scope and technical enforcement were not verified; the older
  hosted-only characterization is therefore too broad.
- **Allie differentiation:** Treat contrast findings as one evidence class within
  a broader criterion/surface ledger and governed release decision.
- **Sources:** [Contrast Advisor](https://applitools.com/solutions/accessibility-testing/),
  [Contrast Advisor docs](https://applitools.com/docs/eyes/concepts/test-execution/accessibility-testing)

## Differentiation Bets

1. **Evidence packet as the product interface.** Every scanner, agent, browser
   run, manual review, waiver, and release projection should project into the
   same versioned packet graph.
2. **Replay before enforcement.** Generated or agentic findings remain advisory
   until replayed through deterministic/scripted evidence or promoted by human
   attestation.
3. **Criterion-by-surface visibility.** The report starts from WCAG criteria and
   product surfaces, not from whatever a scanner happened to find.
4. **Local-first and host-agnostic.** GitHub, Azure, browser clouds, and hosted
   dashboards are adapters over local artifacts, not the only way to use Allie.
5. **Governed AI.** Vision and LLM output should have model metadata, prompt and
   response artifacts, redaction receipts, confidence, and promotion state.
6. **No false legal guarantee.** Allie reports evidence, gaps, status,
   confidence, and residual human review needs.

## Landscape Review Checklist

Run this checklist before V1 positioning changes, hosted/dashboard work, pricing
or packaging decisions, and after any real-app dogfood run:

- Complete the primary-source review and evidence ledger before updating
  `Last reviewed` to the actual inspection date. Do not refresh it only to pass
  the freshness gate.
- Add or remove vendors only with a source URL and an Allie differentiation
  note.
- For every entry, re-check automation depth, standards claims, CI/PR
  integration, artifacts/replay, manual workflow, AI claims, and governance
  posture.
- Mark claims as public-source observations, not legal or procurement advice.
- Run `npm run landscape:smoke` and keep the output with the PR evidence.

## Roadmap Prioritization Rules

- Prefer work that strengthens Allie's packet/replay/release-governance position
  over scanner-rule parity.
- Treat browser clouds, visual AI, and enterprise dashboards as adapters or
  projections until the local evidence contract is strong.
- Add AI/agentic capability only when it carries redaction, model metadata,
  confidence, promotion state, and human-review semantics.
- Do not build hosted dashboards before the host-agnostic CLI and artifact
  contract are easy to dogfood locally.
- If a competitor already solves a proposed workflow well, backlog the Allie
  version only when it materially improves provenance, replay, privacy, or
  standards drilldown.

## Watchlist

- Deque guided-test replay boundaries, automated IGTs, and export/provenance
  contracts; distinguish documented reuse from demonstrated revision replay.
- Evinced SDK failure policy, optional uploads, offline authentication, and
  product-specific retention rather than treating CI blocking as absent.
- Level CI, Vispero ARC, Siteimprove, and Stark evidence models, including
  deployment-specific AI data flows and machine-readable exports.
- BrowserStack assertions and manual-gap summaries, Sauce integration versions,
  and Applitools local-processing claims and contrast-only boundaries.
- WAVE, IBM Equal Access, Accessibility Insights, Pa11y, and W3C ACT rules as
  open or neutral sources for evidence and coverage discipline.

## Review Cadence

Refresh this file before major V1 positioning changes, before hosted/dashboard
work, after dogfood in a real app, and at least monthly while Allie is actively
being shaped.
