# Vox Grammar SSOT

This document defines the canonical vocabulary for the Vox programming language. Both `tree-sitter-vox` and `vox-vscode/syntaxes/vox.tmLanguage.json` must align with these tokens.

## Keywords

### Control Flow
`if`, `else`, `for`, `while`, `match`, `in`, `and`, `or`, `not`, `ret`, `with`, `import`

### Declaration
`fn`, `let`, `mut`, `type`, `actor`, `workflow`, `activity`, `message`, `http`, `routes`, `style`

### Other
`to`, `on`, `is`, `spawn`, `state`, `bind`

## Primitive Types
`int`, `str`, `bool`, `Unit`, `Element`

## Collection Types
`List`, `Map`, `Set`, `Result`, `Option`, `Id`

## Constants
`true`, `false`, `None`, `Ok`, `Err`, `Some`

## Decorators
`@component`, `@test`, `@server`, `@table`, `@index`, `@mcp.tool`, `@mcp.resource`, `@query`, `@mutation`, `@action`, `@v0`, `@skill`, `@agent_def`, `@deprecated`, `@pure`, `@require`, `@storage`

## Operators
`->`, `|>`, `==`, `!=`, `<=`, `>=`, `<`, `>`, `=`, `+=`, `-=`, `+`, `-`, `*`, `/`, `%`, `?`, `?.`

## Comments
- Double slash: `//` (TextMate preferred)
- Line-start hash: `#` (Indentation sensitive/Legacy/SSOT)
