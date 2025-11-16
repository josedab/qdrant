# RFC-0006: Improved Error Messages

**Status:** Draft
**Category:** Quick Win
**Estimated Effort:** 4-6 dev-days

## Summary
Enhance error messages with context and actionable guidance.

## Examples

### Before
```
Error: Validation error
```

### After
```
Error: Vector dimension mismatch
  Expected: 384 (configured for collection 'products')
  Received: 512
  
  Hint: Ensure your embedding model matches the collection configuration.
  See: https://qdrant.tech/docs/troubleshooting#dimension-mismatch
```

## Implementation
1. Structured error types with context
2. Error formatter with hints
3. Documentation links

**Effort:** 4-6 dev-days
