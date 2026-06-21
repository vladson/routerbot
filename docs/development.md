# Development

Keep changes small and reviewable.

Do not introduce external integrations before interfaces and tests exist. Add tests
for every new domain or application behavior, update examples when configuration
changes, and keep feature flags explicit.

Before completing a stage, run:

```bash
./scripts/check.sh
```
