# 041 - Rename package folder to dre

## User Story

As a maintainer working in this codebase, I want the package folder renamed from `sketch` to `dre`, so the project structure and imports reflect the tool's actual name.

## Acceptance Criteria

- The `sketch/` folder is renamed to `dre/`.
- All imports across the codebase (`sketch.layout`, `sketch.render`, etc.) are updated to `dre.layout`, `dre.render`, etc.
- The app still runs and the full test suite still passes after the rename.

## Technical Design

