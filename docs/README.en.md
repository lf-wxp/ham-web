# Documentation Index (English)

[中文](README.md) | English | [Project README](../README.en.md)

> This page is an English translation of the authoritative [`docs/README.md`](README.md). The Chinese index remains the single source of truth for the documentation map, and detailed specifications are currently maintained in Chinese. This translation helps English readers find the right document; it does not create a second index. `cargo make readme-check` requires every document under `docs/` to be linked from the Chinese index.

## Common tasks

Only section-level jumps are listed here. The target documents contain the details; if a task is missing, use the category tables below.

| What you want to do | Where to go |
| --- | --- |
| Change the UI | First identify the skin: Modern UI → [`ui/modern.md`](ui/modern.md), Pixel UI → [`ui/pixel.md`](ui/pixel.md). For components and props → [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md) |
| See an interactive UI-spec example | The in-app `/ui-spec` page (navigation: Advanced · About). Both skins have their own seven sections (components, icons, palette, typography, motion, states), each reading that skin's design tokens |
| Page skeleton / content width / page header | [`ui/pixel.md`](ui/pixel.md), section 4 · [`ui/modern.md`](ui/modern.md), section 3 |
| Values for radii / spacing / type scale / status colors | [`ui/modern.md`](ui/modern.md), section 2 |
| Overlay enter/exit motion; hover, disabled, and error states | [`ui/motion.md`](ui/motion.md), sections five and six |
| Pick a page signature for a new route | [`ui/interaction.md`](ui/interaction.md), section 3 |
| Add or remove `md-*` classes | [`ui/modern-classes.md`](ui/modern-classes.md) (hand-maintained; missing entries are blocked by the guard) |
| Native elements that bypass shared components | [`ui/pixel.md`](ui/pixel.md), section 10 |
| Change interface copy or translations | [`data/i18n.md`](data/i18n.md) |
| Validate UI changes or debug red e2e tests | [`engineering/testing.md`](engineering/testing.md) |
| What to run before committing | [`engineering/code-quality.md`](engineering/code-quality.md), section 2 |
| Add a decoder or Worker error code | [`domain/decode-worker-protocol.md`](domain/decode-worker-protocol.md) |
| Add a local-storage key | [`reference/storage.md`](reference/storage.md), and classify it in `every_documented_key_is_classified` in `core/remote_sync.rs` |
| Work on sync, account APIs, or database tables | [`domain/account-sync.md`](domain/account-sync.md) |
| Understand a design decision | [`ui/decisions.md`](ui/decisions.md), including the origins of the color, geometry, and typography language |

## Documentation categories

Directories are organized by the reader’s question, not by document type.

| Directory | Question it answers | Nature |
| --- | --- | --- |
| [`ui/`](#ui--two-ui-skins) | How to write the interface | Specifications |
| [`engineering/`](#engineering--engineering) | How to develop, test, build, and deploy; code-quality standards | Specifications + guides |
| [`data/`](#data--data-and-content) | How external data, question banks, explanations, and copy are maintained | Specifications + workflows |
| [`reference/`](#reference--quick-reference) | Which routes, storage keys, APIs, and features exist | Quick reference; often tool-checked |
| [`domain/`](#domain--domain-features) | Protocols, definitions, and acceptance criteria for a domain | Specifications |
| [`design/`](#design--design-boards) | What the standalone S9 CONSOLE design boards look like and how to change/export them | Design-board specs + proposals; **not product code constraints** |

### UI / two UI skins

| Document | Contents | When to read it |
| --- | --- | --- |
| [`ui/modern.md`](ui/modern.md) | Modern UI specification: tokens, layout, component catalog, page-writing rules, accessibility, guards, anti-patterns | Changing the Modern UI |
| [`ui/pixel.md`](ui/pixel.md) | Pixel UI specification: visual baseline, decision table, page skeleton, component API contracts, exceptions, self-checks, acceptance checklist | Changing the Pixel UI |
| [`ui/decisions.md`](ui/decisions.md) | Current design trade-offs: two independent kits, the kit contract, how much divergence is allowed, the style switch, validation boundaries, reference implementations, and the visual language’s origins | Understanding why a rule exists |
| [`ui/motion.md`](ui/motion.md) | Motion specification: principles, token sets for both skins, overlay presence/exit, control state matrices | Writing animation, overlays, or control states |
| [`ui/interaction.md`](ui/interaction.md) | Page-level interactions for Modern UI: action classes, route transitions, page signatures, “uniform vs. individual” | Adding page motion or choosing a route signature |
| [`ui/modern-classes.md`](ui/modern-classes.md) | `md-*` class index, hand-maintained and checked bidirectionally by `check-structure` | Finding a class’s owner or whether it is still used |

Code-adjacent documentation: [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md) is the component API catalog, placed beside the code so it is visible while editing components.

### Engineering / engineering

| Document | Contents | When to read it |
| --- | --- | --- |
| [`engineering/architecture.md`](engineering/architecture.md) | Technology stack, full directory tree, crate layers, dependency direction | Deciding where code belongs |
| [`engineering/development.md`](engineering/development.md) | Contributor guide: code style, pre-commit checks, CI overview | First time changing code |
| [`engineering/testing.md`](engineering/testing.md) | Playwright: assertion rules, coverage, run modes, test conventions, coverage accounting, local troubleshooting | Writing, running, or debugging e2e tests |
| [`engineering/tasks.md`](engineering/tasks.md) | Complete `cargo make` task list | Finding a task |
| [`engineering/deployment.md`](engineering/deployment.md) | Local development, production builds, Docker, environment variables, Web Push | Building and deploying |
| [`engineering/code-quality.md`](engineering/code-quality.md) | Code quality: layer boundaries, single source of truth, gate overview, issue severity, evidence rules, scoped checklists | Before commits or during review |
| [`engineering/review-2026-11-27-modern-ui-spec.md`](engineering/review-2026-11-27-modern-ui-spec.md) | One-off review record: the 15 issues found in the Modern UI spec page and home bank picker, how they were fixed, and the regression log | Tracing why this round was fixed this way |
| [`engineering/performance.md`](engineering/performance.md) | Performance: measurement method, thresholds, baselines, guarded rules, evaluated-and-rejected options, caches, bundle budget | Investigating slow pages or caching |
| [`engineering/licensing.md`](engineering/licensing.md) | Licenses, third-party notices, GPL exceptions, distributor obligations | Adding dependencies, fonts, or distributing builds |

### Data / data and content

| Document | Contents | When to read it |
| --- | --- | --- |
| [`data/data-sync.md`](data/data-sync.md) | External data sync: sources, outputs, provenance-stamp schema, freshness matrix, commands, CI classification | Changing data sources or debugging sync checks |
| [`data/data-pipeline.md`](data/data-pipeline.md) | Building question-bank datasets and adding a question-bank version | When the official question bank changes |
| [`data/explanations.md`](data/explanations.md) | Explanation maintenance: count missing entries, merge, enhance, write | Bulk-editing explanations |
| [`data/explanations-style.md`](data/explanations-style.md) | Writing rules for question explanations | Writing one explanation |
| [`data/typography.md`](data/typography.md) | Question-text typography and content fingerprints | Changing question text |
| [`data/i18n.md`](data/i18n.md) | Interface i18n: key rules, plurals, three-language gate, acceptance | Changing any interface string |

### Reference / quick reference

| Document | Contents | When to read it |
| --- | --- | --- |
| [`reference/features.md`](reference/features.md) | Complete feature list; the expanded version of the README feature overview | Checking whether a feature exists and its constraints |
| [`reference/routes.md`](reference/routes.md) | Full route table and parameterized pages, derived from the registry | Finding a path |
| [`reference/storage.md`](reference/storage.md) | Local-storage keys and constraints for both automatic sync channels | Reading or adding a storage key |
| [`reference/api.md`](reference/api.md) | Backend interfaces: same-origin proxy and open `/api/v1/*` API | Integrating with the backend |

### Domain / domain features

| Document | Contents | When to read it |
| --- | --- | --- |
| [`domain/decode-worker-protocol.md`](domain/decode-worker-protocol.md) | Decoder Worker protocol: envelope, error codes, wasm glue contract, addition checklist | Changing decoders |
| [`domain/hardware-verification.md`](domain/hardware-verification.md) | Hardware acceptance: unit tests and fake serial ports only count as “pending”; real-device records count as “verified” | Changing serial ports or external formats |
| [`domain/antenna-coverage.md`](domain/antenna-coverage.md) | Directional gain × DXCC entities; the metric is directional gain, not coverage or reachability | Changing antenna analysis |
| [`domain/pwa.md`](domain/pwa.md) | PWA: Service Worker generation, caching strategy, update prompt, icons | Changing offline or cache behavior |
| [`domain/account-sync.md`](domain/account-sync.md) | Online accounts and sync channels: coexistence with custom remotes, baselines and conflicts, SQLite schema, security, Docker persistence | Changing sync, accounts, or deployment volumes |

### Design / design boards

These boards are independent of product code. They constrain the boards themselves, **not** the current implementation in `crates/app/`; product code follows `ui/`. Do not copy design-board values into the other skin.

| Document | Contents | When to read it |
| --- | --- | --- |
| [`design/s9-console-design-spec.md`](design/s9-console-design-spec.md) | S9 CONSOLE design specification: scope, conflict resolution, style conventions, component factories, naming, accessibility, acceptance checklist, self-check script | Changing the board or translating it into UI code |
| [`design/s9-console-design.html`](design/s9-console-design.html) | The runnable design board and the source of truth for the specification above | Viewing the result or comparing values |
| [`design/s9-console-ui-prompts.md`](design/s9-console-ui-prompts.md) | Image-generation prompts expressing visual intent only; values come from the board | Generating concept art |

Also see [`ROADMAP.md`](../ROADMAP.md) (planned work, deliberate non-goals, and gates for new content) and [`images/`](images/) (README screenshots).

## Documentation conventions

1. **One home per fact.** Write each fact in full in one place; elsewhere, add a short summary and a link. Let tools check numbers and lists that can be derived from code; do not count them by hand. If the same fact seems to belong in three files, decide which one is authoritative and turn the others into links.
2. **Keep specifications separate from history.** `ui/`, `engineering/`, `data/`, and `domain/` describe what to do now. Timelines, trial-and-error, and rejected conclusions belong in commit messages or decisions, not specifications. There is no `archive/` directory and no new one is planned.
3. **Organize directories around the reader’s question.** Before adding a document, ask who will read it and when. If it fits no directory, it probably covers two topics and should be split.
4. **Naming.** Use lowercase and hyphens; do not repeat a prefix already expressed by the directory. English translations use the `.en.md` suffix, as in `README.en.md`; the main Chinese document remains authoritative.
5. **References.** Use relative links. In prose, refer to repository-root paths such as `docs/ui/modern.md`. Numbered sections may be referenced with `§N`; the guard verifies that the target section exists.
6. **Adding, moving, or deleting documents.** Choose the right directory → add, change, or remove one line in the Chinese index → use `git mv` for moves → run `cargo make readme-check` and `cargo make check-structure`.
7. **Update documentation in the same commit as the code.** The table below records ownership:

| Document | How it is maintained | When to change it |
| --- | --- | --- |
| `ui/modern.md`, `ui/pixel.md` | Hand-written | Values, components, or writing rules change |
| `ui/modern-classes.md` | Hand-written; no generator | After adding/removing `md-*` classes in `style/modern/*.css`; missing or dead entries are blocked |
| `ui/decisions.md` | Hand-written | Only when a design decision changes |
| `crates/app/src/ui/README.md` | Hand-written | Components or props are added/removed |
| `reference/routes.md` | Updated with `crates/core/src/registry.rs` | Routes are added or changed; count is checked by `readme-check` |
| `reference/features.md` | Hand-written, paired with the README feature overview | Features change |
| `reference/storage.md` | Hand-written | Local-storage keys are added or changed |

## Guards

Which document assertions are machine-checked; everything else relies on human review:

| Guard | What it guarantees |
| --- | --- |
| `cargo make readme-check` | Chinese README counts and task lists; every `docs/` document is linked from the Chinese index; README and AGENTS point to that index; Chinese README section references still resolve; screenshots exist |
| `cargo make check-structure` (document links) | Relative links resolve; `§N` references point to an existing numbered section. It proves existence, not topical accuracy |
| `cargo make check-structure` (class index) | `ui/modern-classes.md` and `style/modern/*.css` match bidirectionally |
| `cargo make check-ui` | Mechanical “must be 0” items from the Pixel UI specification |
| `cargo make i18n-check` · `knowledge-i18n-check` | Three-language keys are complete; knowledge-translation keys do not drift |
| `cargo make data-sync-check` · `data-staleness` | Data provenance stamps are consistent; manually transcribed facts are reviewed on time |

The full gate list and what each failure means are in [`engineering/code-quality.md`](engineering/code-quality.md), section 2.

## Glossary

| Term | Meaning |
| --- | --- |
| Modern UI / Pixel UI | Two coexisting interface skins. Modern UI = `md-*` classes + `--md-*` variables + `kit_modern/` + `style/modern/`; Pixel UI = `pxl-*` and utility classes + unprefixed variables + `components/` + `style/pixel/`. They share no class names or tokens; the only shared piece is the style branch in `crates/app/src/ui/` |
| Kit | A skin’s component set (`kit_modern/` or `components/`); both satisfy the same Kit contract |
| Bridged page | A page without a dedicated Modern UI implementation, adapted from Pixel UI DOM with `legacy-utilities.css` |
| Page signature | The family of page-level actions selected for a route in `interaction.css` |
| Presence | The shared mechanism that keeps an overlay in the DOM long enough to finish its exit animation, then unmounts it (`crate::presence`) |
| Gate | A mechanical guard in `cargo make check`; treat a failure as P0 |
| Provenance stamp | A marker on generated data recording its source and dependency digest |
| Content fingerprint | A hash of normalized question text; explanations, mistakes, and favorites use it to identify questions |
| SSOT | Single source of truth: one authoritative definition, referenced everywhere else |
