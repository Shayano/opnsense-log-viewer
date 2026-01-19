# Story 0.4: TypeScript Build-Test Configuration Alignment

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want TypeScript strict mode consistently enforced across both test execution (Vitest) and production builds (tsc),
So that type safety issues are caught during development rather than failing the production build.

## Acceptance Criteria

**Given** the project has TypeScript strict mode enabled in tsconfig.json (Story 0.1)
**When** I run `npm test`
**Then** all tests pass with full type safety enforcement
**And** Vitest globals (describe, it, expect, beforeEach, afterEach) are properly typed

**When** I run `npm run build`
**Then** the TypeScript compilation succeeds with 0 errors
**And** all type safety rules from tsconfig.json are enforced

**When** I run `npm run build:release`
**Then** both the frontend build and Tauri build complete successfully
**And** no TypeScript errors are present in production code or test files

**Given** existing type safety issues from previous stories
**When** I fix all TypeScript compilation errors
**Then** the following categories are resolved:
- Config possibly undefined errors (connection-indicator.tsx)
- Type mismatches in production code (use-export.ts, enrichment-export.tsx)
- Null vs string|undefined mismatches in tests (use-ip-alias.test.ts)
- Missing test global definitions (beforeEach, afterEach, global)
- Unused variable warnings in test files
- Type casting issues in test mocks

**When** I create or update tsconfig files
**Then** the configuration ensures:
- Test files have access to Vitest global types
- Strict mode is enforced in both production and test contexts
- Type checking happens during test runs (not just at build time)
- No type errors are suppressed or ignored

## Tasks / Subtasks

- [ ] Analyze TypeScript configuration gap between test and build (AC: All criteria)
  - [ ] Document current tsconfig.json configuration (strict: true, include: ["src"])
  - [ ] Document current vitest.config.ts configuration (globals: true)
  - [ ] Identify why Vitest tests pass but `tsc` build fails
  - [ ] Review project-context.md lines 138-142 (TypeScript strict mode requirements)
  - [ ] Categorize 74 build errors by type and priority

- [ ] Fix HIGH priority type safety errors in production code (AC: Type safety in production)
  - [ ] Fix connection-indicator.tsx:39-51 - Handle config possibly undefined (6 errors)
  - [ ] Fix use-export.ts:63 - Correct boolean vs ExportScope type mismatch
  - [ ] Fix enrichment-export.tsx:103 - Resolve string|number vs string for toast.dismiss
  - [ ] Verify fixes don't break existing functionality
  - [ ] Run tests after each fix to ensure no regressions

- [ ] Add Vitest type definitions to TypeScript configuration (AC: Vitest globals typed)
  - [ ] Install @vitest/globals type definitions if not present
  - [ ] Update tsconfig.json or create tsconfig.test.json to include Vitest types
  - [ ] Ensure test files recognize describe, it, expect, beforeEach, afterEach, global
  - [ ] Verify vitest.config.ts globals: true works with TypeScript
  - [ ] Test that both `npm test` and `npm run build` succeed

- [ ] Fix MEDIUM priority test type safety errors (AC: All tests type-safe)
  - [ ] Fix use-ip-alias.test.ts - Resolve 10 null vs string|undefined errors
  - [ ] Fix enrichment-import-service.test.ts - Add global type definitions
  - [ ] Fix other test files missing global definitions (beforeEach, afterEach)
  - [ ] Ensure all test mocks have correct type signatures
  - [ ] Run full test suite to verify 353/353 tests still pass

- [ ] Clean up LOW priority dead code warnings (AC: Zero build warnings)
  - [ ] Remove unused variables in test files (~40 errors)
  - [ ] Fix type casting issues in test mocks (~6 errors)
  - [ ] Clean up unused imports in integration tests
  - [ ] Run ESLint to catch any remaining linting issues
  - [ ] Verify code quality standards from project-context.md lines 304-309

- [ ] Verify build and test alignment (AC: build:release succeeds)
  - [ ] Run `npm test` - expect 353/353 tests passing
  - [ ] Run `npm run build` - expect 0 TypeScript errors
  - [ ] Run `npm run build:release` - expect full build success
  - [ ] Verify no type errors in console output
  - [ ] Confirm alignment with project-context.md strict mode requirements

- [ ] Update Definition of Done checklist (AC: Process improvement)
  - [ ] Add explicit "npm run build must pass" to pre-commit checklist
  - [ ] Document in project-context.md or workflow documentation
  - [ ] Ensure future stories validate both tests AND builds before marking done
  - [ ] Update sprint-status.yaml with story 0-4 completion

## Dev Notes

### Architecture Context

**Root Cause Analysis:**
From project-context.md lines 138-142:
> **TypeScript Strict Mode (MANDATORY):**
> - ✅ Use `strict: true` in tsconfig.json
> - ✅ Explicit return types for functions
> - ✅ No implicit `any` types
> - ❌ DO NOT use `any` unless absolutely necessary (use `unknown` instead)

The issue: tsconfig.json has `"strict": true"` and `"include": ["src"]"`, which means TypeScript compiler checks all `.ts` and `.tsx` files in src/ directory. However, Vitest runs with `globals: true` which injects test globals at runtime, but TypeScript compiler doesn't have type definitions for these globals during build phase.

**Error Categories (74 total):**

**HIGH PRIORITY (Runtime Risk - 9 errors):**
1. connection-indicator.tsx:39-51 - config possibly undefined (6 errors)
2. use-export.ts:63 - boolean passed where ExportScope enum expected
3. enrichment-export.tsx:103 - string|number vs string mismatch

**MEDIUM PRIORITY (Test Type Safety - 22 errors):**
4. use-ip-alias.test.ts - null assigned where string|undefined expected (10 errors)
5. Test files missing global definitions - beforeEach, afterEach, global (12 errors)

**LOW PRIORITY (Dead Code - 43 errors):**
6. Unused variables across test files (~40 errors)
7. Type casting issues in test mocks (~3 errors)

**Critical Files to Fix:**
- src/components/api-status/connection-indicator.tsx:39-51
- src/hooks/use-export.ts:63
- src/services/enrichment-export.tsx:103
- src/hooks/use-ip-alias.test.ts (multiple lines)
- src/services/enrichment-import-service.test.ts
- src/components/filter-sidebar/clear-confirm-dialog.test.tsx
- src/components/filter-sidebar/delete-confirm-dialog.test.tsx
- src/services/enrichment-export.test.tsx

**Technical Approach:**

**Option A - Single tsconfig.json (Simpler):**
- Add Vitest types to existing tsconfig.json
- Ensure @vitest/globals types are installed
- Minimal changes, single source of truth

**Option B - Separate test config (More explicit):**
- Create tsconfig.test.json that extends tsconfig.json
- Update vitest.config.ts to use tsconfig.test.json
- Clearer separation between production and test types
- More configuration overhead

**Recommended:** Option A for simplicity, unless team prefers explicit separation.

**Dependencies:**
- Story 0.1 (Project Scaffolding) - COMPLETE
- Story 0.2 (Test Infrastructure) - COMPLETE
- Story 0.3 (Tailwind CSS) - COMPLETE

**Related Context:**
- project-context.md lines 138-142: TypeScript strict mode rules
- project-context.md lines 246-250: Testing rules (Vitest + co-located tests)
- project-context.md lines 304-309: Code quality and linting rules
- project-context.md lines 348-357: Pre-commit checks (needs build verification added)
- sprint-status.yaml lines 111-183: Shows all stories marked done but build fails

**References:**
- PRD Section 9: Quality Requirements - Type Safety
- Architecture Section 4.5: Frontend Framework - TypeScript Configuration
- UX Design: N/A (infrastructure story)

**Story Context:**
- Epic: 0 (Foundation & Infrastructure)
- Sprint: Retrospective fix (discovered post-completion)
- Urgency: HIGH - Blocks production release
- Impact: ALL future stories depend on working build pipeline

### Implementation Notes

**Type Definition Strategy:**
```typescript
// Vitest globals are injected at runtime via vitest.config.ts globals: true
// TypeScript needs type definitions to recognize them during compilation

// Option 1: Add to tsconfig.json compilerOptions.types
{
  "compilerOptions": {
    "types": ["vitest/globals"]
  }
}

// Option 2: Use /// <reference types="vitest/globals" /> in test files
// (more explicit but requires adding to every test file)
```

**Null Safety Pattern:**
```typescript
// ❌ WRONG - Assigning null where string|undefined expected
const result: string | undefined = null;

// ✅ CORRECT - Use undefined for optional values
const result: string | undefined = undefined;
```

**Config Safety Pattern:**
```typescript
// ❌ WRONG - Assuming config is always defined
const url = config.apiUrl;

// ✅ CORRECT - Guard against undefined
const url = config?.apiUrl;
// OR
if (!config) {
  throw new Error('Config not initialized');
}
const url = config.apiUrl;
```

**Enum Type Safety:**
```typescript
// ❌ WRONG - Boolean where enum expected
const scope = true; // Type: boolean
exportData(scope); // Error: boolean not assignable to ExportScope

// ✅ CORRECT - Use enum value
const scope = ExportScope.Filtered; // Type: ExportScope
exportData(scope); // ✓
```

### Quality Gates

**Pre-Implementation Checklist:**
- [x] Story created with clear acceptance criteria
- [x] Root cause analysis documented
- [x] All 74 errors categorized by priority
- [x] Implementation approach defined
- [ ] Ready for dev-story workflow

**Post-Implementation Validation:**
- [ ] `npm test` passes (353/353 tests)
- [ ] `npm run build` passes (0 TypeScript errors)
- [ ] `npm run build:release` succeeds fully
- [ ] ESLint passes with no warnings
- [ ] All type safety rules enforced
- [ ] Definition of Done updated with build verification

**Test Coverage Impact:**
- Expected: 0 new tests (infrastructure fix)
- Actual: TBD after implementation
- Reason: Fixing existing type errors, not adding functionality

**Performance Impact:**
- Expected: None (configuration change only)
- Risk: Minimal - no runtime behavior changes

### Retrospective Questions

**What caused this gap?**
- Tests validated, but builds not validated before marking stories "done"
- Definition of Done didn't explicitly include "npm run build must pass"
- Vitest isolated from TypeScript compiler created false positive

**How do we prevent recurrence?**
- Add explicit build verification to Definition of Done
- Update pre-commit checklist in project-context.md
- Consider adding `tsc --noEmit` to npm test script
- Ensure sprint-status validation includes build gates

**What did we learn?**
- Test passing ≠ Build passing (different type checking contexts)
- Vitest globals need explicit TypeScript type definitions
- Infrastructure stories are critical foundation for all subsequent work
- Process adherence requires explicit checklists, not assumptions
