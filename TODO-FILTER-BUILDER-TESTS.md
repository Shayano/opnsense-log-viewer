# TODO - Fix Filter Builder Tests (8 failing tests)

**Fichier**: `src/components/filter-builder/filter-builder.test.tsx`
**Priorité**: Moyenne
**Effort estimé**: 30 minutes

## Problème identifié

Les 8 tests échouent avec l'erreur:
```
TestingLibraryElementError: Found multiple elements with the text: Add Filter
```

**Cause**: Le composant contient à la fois:
- Un titre `<h2>Add Filter</h2>`
- Un bouton `<button>Add Filter</button>`

Les tests utilisent `screen.getByText('Add Filter')` qui trouve les deux éléments et échoue.

## Tests affectés

1. ❌ `should add filter to store on submit`
2. ❌ `should disable submit button when form is incomplete`
3. ❌ `should enable submit button when form is complete`
4. ❌ `should reset form after successful submit`
5. ❌ `should show error toast if form is incomplete on submit`
6. ❌ `should handle multiple filter additions`
7. ❌ `should handle filter validation errors`
8. ❌ `should update draft mode when adding filter`

## Solution recommandée

Remplacer:
```typescript
const submitButton = screen.getByText('Add Filter');
```

Par:
```typescript
const submitButton = screen.getByRole('button', { name: 'Add Filter' });
```

Ou alternativement:
```typescript
const submitButton = screen.getByRole('button', { name: /add filter/i });
```

## Fichiers à modifier

1. `src/components/filter-builder/filter-builder.test.tsx` - Lignes affectées:
   - Ligne ~90: `should add filter to store on submit`
   - Ligne ~168: `should disable submit button when form is incomplete`
   - Ligne ~189: `should enable submit button when form is complete`
   - Ligne ~212: `should reset form after successful submit`
   - Ligne ~235: `should show error toast if form is incomplete on submit`
   - Autres lignes similaires pour les 3 autres tests

## Commande pour tester après correction

```bash
npm test -- filter-builder.test.tsx --run
```

## Contexte Phase 2

Ces tests étaient pré-existants. La Phase 2 a ajouté 84 nouveaux tests (tous passants):
- Story 2.3: 45 tests ✅
- Story 3.3: 9 tests ✅
- Story 4.2: 9 tests + 2 bugs corrigés ✅
- Story 1.6: 8 tests ✅
- Story 4.1: 13 tests ✅

**Total suite de tests**: 345/353 passants (98%)
